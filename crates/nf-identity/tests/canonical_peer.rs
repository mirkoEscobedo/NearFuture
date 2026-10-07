use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::{codec, model::*, peer::validate_canonical_peer};
#[path = "canonical_peer_support/mod.rs"]
mod support;
use support::corpus;

#[test]
fn independent_canonical_peer_fixtures_are_admitted() {
    for name in ["client-peer", "server-peer"] {
        assert_eq!(validate_canonical_peer(&corpus(name)), Ok(()));
    }
}

#[test]
fn bounded_opaque_peer_is_rejected_only_by_the_new_syntax_port() {
    let opaque = vec![0xff];
    let founder = PublicIdentity {
        account: AccountId::from_bytes([1; 16]),
        device: DeviceId::from_bytes([2; 16]),
        account_key: [3; 32],
        device_key: [4; 32],
        peer: opaque.clone(),
    };
    let scope = Scope {
        universe: UniverseId::from_bytes([5; 16]),
        history: HistoryId::from_bytes([6; 16]),
    };
    let state = MembershipState::bootstrap(scope, &founder).unwrap();
    let bytes = codec::encode_state(&state).unwrap();
    assert_eq!(codec::decode_state(&bytes).unwrap(), state);
    assert_eq!(
        validate_canonical_peer(&opaque),
        Err(IdentityError::Malformed)
    );
}

#[test]
fn envelope_limits_and_trailing_or_unsupported_syntax_refuse() {
    assert_eq!(validate_canonical_peer(&[]), Err(IdentityError::Limit));
    assert_eq!(
        validate_canonical_peer(&[0; 129]),
        Err(IdentityError::Limit)
    );
    let mut trailing = corpus("client-peer");
    trailing.push(0);
    assert_eq!(
        validate_canonical_peer(&trailing),
        Err(IdentityError::Malformed)
    );
    assert_eq!(
        validate_canonical_peer(&[1, 1, 7]),
        Err(IdentityError::Malformed)
    );
}

#[test]
fn independent_noncanonical_prefix_fixture_refuses() {
    let record = corpus("proof-peer-noncanonical-prefix");
    // Normative DeviceProof297 ends with challenge32 and signature64.
    // Its Peer129 starts after scope32/account16/device16/frontier8.
    let peer_offset = record.len() - 297 + 72;
    let width = usize::from(record[peer_offset]);
    assert!((1..=128).contains(&width));
    assert_eq!(
        validate_canonical_peer(&record[peer_offset + 1..peer_offset + 1 + width]),
        Err(IdentityError::Malformed)
    );
}

#[test]
fn maintained_parser_edges_do_not_claim_embedded_key_or_hash_width() {
    assert_eq!(validate_canonical_peer(&[0, 0]), Ok(()));
    assert_eq!(validate_canonical_peer(&[0x12, 0]), Ok(()));
    let mut largest_sha_shape = vec![0x12, 64];
    largest_sha_shape.extend_from_slice(&[9; 64]);
    assert_eq!(validate_canonical_peer(&largest_sha_shape), Ok(()));
    let mut largest_inline = vec![0, 42];
    largest_inline.extend_from_slice(&[10; 42]);
    assert_eq!(validate_canonical_peer(&largest_inline), Ok(()));
    let mut over_inline = vec![0, 43];
    over_inline.extend_from_slice(&[10; 43]);
    assert_eq!(
        validate_canonical_peer(&over_inline),
        Err(IdentityError::Malformed)
    );
}
