use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::{
    auth::HandshakeContext,
    records::{Lane, PeerContext, PeerLimits},
};
use sha2::{Digest, Sha256};
#[test]
fn maintained_proof_transcript_binds_roles_peers_policy_limits_nonce_stage_and_frontier() {
    let c = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let s = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let h = HandshakeContext {
        lane: Lane::Control,
        client_peer: c,
        server_peer: s,
        client_account: AccountId::from_bytes([1; 16]),
        client_device: DeviceId::from_bytes([2; 16]),
        server_account: AccountId::from_bytes([3; 16]),
        server_device: DeviceId::from_bytes([4; 16]),
        context: PeerContext {
            session: [5; 16],
            scope: Scope {
                universe: UniverseId::from_bytes([6; 16]),
                history: HistoryId::from_bytes([7; 16]),
            },
            ruleset: [8; 32],
            content: [9; 32],
        },
        client_nonce: [10; 32],
        server_nonce: [11; 32],
        required: 1,
        optional: 2,
        server_available: 3,
        selected_caps: 3,
        offered: PeerLimits::default(),
        server_limits: PeerLimits::default(),
        selected: PeerLimits::default(),
    };
    let mut expected = b"NF-PEER-AUTH-1\0".to_vec();
    expected.extend([1]);
    expected.extend(1u16.to_le_bytes());
    expected.push(1);
    for peer in [c, s] {
        let p = peer.to_bytes();
        expected.push(p.len() as u8);
        expected.extend(&p);
        expected.resize(expected.len() + 128 - p.len(), 0);
    }
    for v in [1u8, 2, 3, 4, 5, 6, 7] {
        expected.extend([v; 16]);
    }
    for v in [8u8, 9, 10, 11] {
        expected.extend([v; 32]);
    }
    for v in [1u32, 2, 3, 3] {
        expected.extend(v.to_le_bytes());
    }
    for _ in 0..3 {
        for v in [4096u32, 9216, 8192, 65536, 131072] {
            expected.extend(v.to_le_bytes());
        }
        for v in [16u16, 4, 8] {
            expected.extend(v.to_le_bytes());
        }
    }
    expected.extend(12u64.to_le_bytes());
    assert_eq!(expected.len(), 619);
    assert_eq!(
        h.challenge(1, 12).unwrap(),
        <[u8; 32]>::from(Sha256::digest(&expected))
    );
    assert_ne!(h.challenge(1, 12).unwrap(), h.challenge(2, 12).unwrap());
    assert_ne!(h.challenge(1, 12).unwrap(), h.challenge(1, 13).unwrap());
    let mut changed = h.clone();
    changed.server_nonce[0] ^= 1;
    assert_ne!(
        h.challenge(1, 12).unwrap(),
        changed.challenge(1, 12).unwrap()
    );
    changed = h.clone();
    changed.lane = Lane::Bulk;
    assert_eq!(
        changed.challenge(1, 12),
        Err(nf_transport::PeerError::Unsupported)
    );
    changed = h.clone();
    changed.selected.control_items = 15;
    assert!(changed.challenge(1, 12).is_err());
}
