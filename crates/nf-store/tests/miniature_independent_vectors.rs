mod miniature_independent_support;
use miniature_independent_support::{hex, vector};
use nf_contract::identity::*;
use nf_identity::model::PublicIdentity;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
#[test]
fn public_bootstrap_preimage_matches_independent_node_fields_and_digest() {
    let world = decode_miniature_snapshot(&vector("envelope", "genesis")[49..1260]).unwrap();
    let owner = PublicIdentity {
        account: AccountId::from_bytes([4; 16]),
        device: DeviceId::from_bytes([33; 16]),
        peer: vec![1, 2, 3],
        account_key: hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .try_into()
            .unwrap(),
        device_key: hex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c")
            .try_into()
            .unwrap(),
    };
    let expected = vector("bootstrap", "creation-binding");
    let actual = bootstrap_binding_preimage(&world, [7; 32], &owner).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 1367);
    let mut changed = owner.clone();
    changed.device = DeviceId::from_bytes([34; 16]);
    assert_ne!(
        Sha256::digest(bootstrap_binding_preimage(&world, [7; 32], &changed).unwrap()),
        Sha256::digest(&expected)
    );
    changed.peer.clear();
    assert!(bootstrap_binding_preimage(&world, [7; 32], &changed).is_err());
}
#[test]
fn actual_generated_signer_and_atomic_genesis_match_independent_outer_envelope() {
    let (state, owner, mut signer) = miniature_independent_support::membership();
    let bytes = nf_identity::codec::encode_state(&state).unwrap();
    let golden = vector("envelope", "genesis");
    let world = decode_miniature_snapshot(&golden[49..1260]).unwrap();
    let metadata = world.metadata();
    let spec = MiniatureGenesisSpec {
        genesis: world.component().genesis(),
        aggregate: metadata.aggregate,
        provider: metadata.provider,
        provider_aggregate: metadata.provider_aggregate,
    };
    let policy = BootstrapPolicy {
        scope: state.scope,
        membership_digest: Sha256::digest(&bytes).into(),
        owner,
        controllers: spec.genesis.accounts,
        auth: AuthConfig::default(),
    };
    let scratch = miniature_independent_support::Scratch::new();
    let path = scratch.database();
    let store = MiniatureStore::create(&path, spec, &bytes, policy, &mut signer).unwrap();
    assert_eq!(store.snapshot_bytes().unwrap(), golden);
    let minima = store.known_frontiers().unwrap();
    drop(store);
    let reopened = MiniatureStore::open_existing(&path, minima, AuthConfig::default()).unwrap();
    assert_eq!(reopened.snapshot_bytes().unwrap(), golden);
    drop(reopened);
}
#[test]
fn owned_scratch_is_unique_and_removed_when_an_assertion_unwinds() {
    let first = miniature_independent_support::Scratch::new();
    let second = miniature_independent_support::Scratch::new();
    assert_ne!(first.database(), second.database());
    let path = first.database();
    let directory = path.parent().unwrap().to_owned();
    let failed = std::panic::catch_unwind(move || {
        let _owned = first;
        std::fs::write(path, b"owned fixture").unwrap();
        panic!("deliberate test fixture unwind");
    });
    assert!(failed.is_err());
    assert!(!directory.exists());
    let second_directory = second.database().parent().unwrap().to_owned();
    drop(second);
    assert!(!second_directory.exists());
}

#[test]
fn independent_malformed_envelopes_are_rejected_with_matching_outer_digests() {
    miniature_independent_support::admission::reject_malformed_history();
}
#[test]
fn real_issued_claim_and_lease_match_independent_context_and_consume_first_attempt() {
    miniature_independent_support::challenge::issued_transcripts_and_first_attempt_are_bound();
}
#[test]
fn actual_extra_schema_objects_and_inactive_mirrors_are_rejected() {
    miniature_independent_support::schema::schema_and_mirrors_fail_closed();
}
