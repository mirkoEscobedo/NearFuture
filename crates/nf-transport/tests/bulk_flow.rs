#[path = "support/bulk_pair.rs"]
mod bulk_pair;
#[path = "support/community.rs"]
mod community;
use community::Community;
use nf_identity::{
    model::{DeviceRevocation, Roles},
    rotation::revocation_digest,
};
use nf_transport::{
    PeerError,
    bulk::BulkResult,
    records::{BulkDescriptor, PeerBody},
};
use sha2::{Digest, Sha256};
use std::time::Instant;
fn descriptor() -> BulkDescriptor {
    BulkDescriptor {
        transfer: [9; 16],
        total: 6,
        chunks: 2,
        digest: Sha256::digest(b"abcdef").into(),
    }
}
#[test]
fn ordered_bulk_proof_ready_progress_and_digest_do_not_install_world_state() {
    let c = Community::new(Roles::PLAYER);
    let (mut client, mut server, cp, sp, id) = bulk_pair::pair(&c);
    let now = Instant::now();
    let begin = client.begin(descriptor(), &c.state, now).unwrap();
    let challenge = server.begin(begin, cp, id, &c.state, now).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &c.state, &c.client_device, now)
        .unwrap();
    let ready = server
        .prove(proof.clone(), cp, id, &c.state, &c.server_device, now)
        .unwrap();
    assert_eq!(
        server.prove(proof, cp, id, &c.state, &c.server_device, now),
        Err(PeerError::Replay)
    );
    client.ready(ready.clone(), sp, id, &c.state, now).unwrap();
    let chunk = client.chunk(b"abc", &c.state, now).unwrap();
    let progress = server
        .chunk(chunk, cp, id, &c.state, &c.server_device, now)
        .unwrap();
    assert_eq!(
        client
            .progress(progress.clone(), sp, id, &c.state, now)
            .unwrap(),
        BulkResult::Progress {
            next_index: 1,
            accepted_total: 3
        }
    );
    let chunk = client.chunk(b"def", &c.state, now).unwrap();
    let final_reply = server
        .chunk(chunk, cp, id, &c.state, &c.server_device, now)
        .unwrap();
    assert_eq!(
        client.progress(final_reply, sp, id, &c.state, now).unwrap(),
        BulkResult::Verified {
            transfer: [9; 16],
            total: 6,
            digest: descriptor().digest
        }
    );
    assert_eq!(
        client.progress(progress, sp, id, &c.state, now),
        Err(PeerError::Replay)
    );
    assert_eq!(
        client.ready(ready, sp, id, &c.state, now),
        Err(PeerError::Replay)
    );
}
#[test]
fn signed_progress_tamper_and_old_progress_cannot_authorize_next_chunk() {
    for tamper in [false, true] {
        let c = Community::new(Roles::PLAYER);
        let (mut client, mut server, cp, sp, id) = bulk_pair::pair(&c);
        let now = Instant::now();
        let begin = client.begin(descriptor(), &c.state, now).unwrap();
        let challenge = server.begin(begin, cp, id, &c.state, now).unwrap();
        let proof = client
            .challenge(challenge, sp, id, &c.state, &c.client_device, now)
            .unwrap();
        let ready = server
            .prove(proof, cp, id, &c.state, &c.server_device, now)
            .unwrap();
        client.ready(ready, sp, id, &c.state, now).unwrap();
        let chunk = client.chunk(b"abc", &c.state, now).unwrap();
        let mut progress = server
            .chunk(chunk, cp, id, &c.state, &c.server_device, now)
            .unwrap();
        if tamper {
            if let PeerBody::BulkProgress { accepted_total, .. } = &mut progress.body {
                *accepted_total = 4;
            }
            assert_eq!(
                client.progress(progress, sp, id, &c.state, now),
                Err(PeerError::Unauthorized)
            );
        } else {
            client
                .progress(progress.clone(), sp, id, &c.state, now)
                .unwrap();
            let _ = client.chunk(b"def", &c.state, now).unwrap();
            assert_eq!(
                client.progress(progress, sp, id, &c.state, now),
                Err(PeerError::Replay)
            );
        }
        assert_eq!(client.chunk(b"x", &c.state, now), Err(PeerError::Replay));
    }
}
#[test]
fn current_revocation_fences_bulk_chunk_and_same_signed_progress() {
    for revoke_server in [false, true] {
        let c = Community::new(Roles::PLAYER);
        let (mut client, mut server, cp, sp, id) = bulk_pair::pair(&c);
        let now = Instant::now();
        let begin = client.begin(descriptor(), &c.state, now).unwrap();
        let challenge = server.begin(begin, cp, id, &c.state, now).unwrap();
        let proof = client
            .challenge(challenge, sp, id, &c.state, &c.client_device, now)
            .unwrap();
        let ready = server
            .prove(proof, cp, id, &c.state, &c.server_device, now)
            .unwrap();
        client.ready(ready, sp, id, &c.state, now).unwrap();
        let chunk = client.chunk(b"abc", &c.state, now).unwrap();
        let revoke = DeviceRevocation {
            scope: c.state.scope,
            issuer: c.state.owner,
            device: if revoke_server {
                c.server.device
            } else {
                c.client.device
            },
            frontier: c.state.revision,
        };
        let current = c
            .state
            .revoke_device(&revoke, &c.server_account.sign(&revocation_digest(&revoke)))
            .unwrap();
        if revoke_server {
            let progress = server
                .chunk(chunk, cp, id, &c.state, &c.server_device, now)
                .unwrap();
            assert_eq!(
                client.progress(progress, sp, id, &current, now),
                Err(PeerError::Unauthorized)
            );
        } else {
            assert_eq!(
                server.chunk(chunk, cp, id, &current, &c.server_device, now),
                Err(PeerError::Unauthorized)
            );
        }
    }
}

#[test]
fn reordered_and_duplicated_chunks_poison_the_exact_transfer_instead_of_silently_dropping() {
    for duplicate in [false, true] {
        let c = Community::new(Roles::PLAYER);
        let (mut client, mut server, cp, sp, id) = bulk_pair::pair(&c);
        let now = Instant::now();
        let begin = client.begin(descriptor(), &c.state, now).unwrap();
        let challenge = server.begin(begin, cp, id, &c.state, now).unwrap();
        let proof = client
            .challenge(challenge, sp, id, &c.state, &c.client_device, now)
            .unwrap();
        let ready = server
            .prove(proof, cp, id, &c.state, &c.server_device, now)
            .unwrap();
        client.ready(ready, sp, id, &c.state, now).unwrap();
        let mut chunk = client.chunk(b"abc", &c.state, now).unwrap();
        if duplicate {
            server
                .chunk(chunk.clone(), cp, id, &c.state, &c.server_device, now)
                .unwrap();
        } else if let PeerBody::BulkChunk { index, .. } = &mut chunk.body {
            *index = 1;
        }
        assert_eq!(
            server.chunk(chunk.clone(), cp, id, &c.state, &c.server_device, now),
            Err(PeerError::Replay)
        );
        assert_eq!(
            server.chunk(chunk, cp, id, &c.state, &c.server_device, now),
            Err(PeerError::Replay)
        );
    }
}
