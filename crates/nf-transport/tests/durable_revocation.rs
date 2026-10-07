#[path = "support/community.rs"]
mod community;
#[path = "support/query_pair.rs"]
mod query_pair;
#[path = "support/scratch.rs"]
mod scratch;
use community::Community;
use nf_contract::identity::RequestId;
use nf_identity::{
    model::{DeviceRevocation, MembershipRepository, Roles},
    rotation::revocation_digest,
};
use nf_transport::{PeerError, records::RetainedPhase};
use std::time::Instant;
#[test]
fn server_reloads_durable_revocation_before_protected_lookup() {
    let c = Community::new(Roles::PLAYER);
    let scratch = scratch::Scratch::new();
    let (mut client, mut server, cp, sp, id) = query_pair::pair(&c, &scratch.0);
    let now = Instant::now();
    let begin = client.begin(RequestId::from_bytes([9; 16]), now).unwrap();
    let challenge = server.begin(begin, cp, id, now).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &c.client_device, now)
        .unwrap();
    let revoke = DeviceRevocation {
        scope: c.state.scope,
        issuer: c.state.owner,
        device: c.client.device,
        frontier: c.state.revision,
    };
    let next = c
        .state
        .revoke_device(&revoke, &c.server_account.sign(&revocation_digest(&revoke)))
        .unwrap();
    server
        .owner_store_mut()
        .commit_membership(Some(1), &next)
        .unwrap();
    assert_eq!(
        server.prove(proof, cp, id, &c.server_device, now),
        Err(PeerError::Unauthorized)
    );
}
#[test]
fn client_reloads_durable_server_revocation_before_accepting_same_numeric_status() {
    let c = Community::new(Roles::PLAYER);
    let scratch = scratch::Scratch::new();
    let (mut client, mut server, cp, sp, id) = query_pair::pair(&c, &scratch.0);
    let now = Instant::now();
    let request = RequestId::from_bytes([9; 16]);
    let begin = client.begin(request, now).unwrap();
    let challenge = server.begin(begin, cp, id, now).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &c.client_device, now)
        .unwrap();
    let response = server.prove(proof, cp, id, &c.server_device, now).unwrap();
    assert!(matches!(
        &response.body,
        nf_transport::records::PeerBody::RetainedStatus {
            phase: RetainedPhase::UnknownRequest,
            ..
        }
    ));
    let revoke = DeviceRevocation {
        scope: c.state.scope,
        issuer: c.state.owner,
        device: c.server.device,
        frontier: c.state.revision,
    };
    let next = c
        .state
        .revoke_device(&revoke, &c.server_account.sign(&revocation_digest(&revoke)))
        .unwrap();
    client
        .owner_store_mut()
        .commit_membership(Some(1), &next)
        .unwrap();
    assert_eq!(
        client.reply(response, sp, id, now),
        Err(PeerError::Unauthorized)
    );
}
