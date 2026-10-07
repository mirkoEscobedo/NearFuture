mod driver_support;
use nf_identity::{
    keys::SecretSeed, model::*, private_storage::PrivateVault, rotation::rotation_digest,
};
use nf_world_driver::VaultSigner;
#[test]
fn existing_staged_key_selected_only_after_verified_current_membership_rotation() {
    let f = driver_support::Fixture::new();
    let o = &f.options;
    let vault = PrivateVault::open(&o.vault, &o.game_save_root).unwrap();
    let local = vault.load_identity(o.policy.owner.peer.clone()).unwrap();
    let old = MembershipState::bootstrap(o.policy.scope, &local.public).unwrap();
    let fresh = SecretSeed::generate().unwrap();
    vault.stage_rotation_key(&fresh).unwrap();
    let change = DeviceRotation {
        scope: o.policy.scope,
        account: local.public.account,
        device: local.public.device,
        frontier: 0,
        new_key: fresh.public_key(),
        new_peer: vec![9],
    };
    let digest = rotation_digest(&change).unwrap();
    let next = old
        .rotate_device(
            &change,
            &local.account_key.sign(&digest),
            &fresh.sign(&digest),
        )
        .unwrap();
    let mut public = local.public.clone();
    public.device_key = change.new_key;
    public.peer = change.new_peer;
    let signer = VaultSigner::open(&o.vault, &o.game_save_root, o.policy.scope, &public)
        .expect("load reviewed immutable staged key for current membership");
    let template = DeviceProof {
        scope: o.policy.scope,
        account: public.account,
        device: public.device,
        frontier: 1,
        peer: public.peer.clone(),
        challenge: [5; 32],
        signature: [0; 64],
    };
    let proof = signer.sign(&template).unwrap();
    assert_eq!(
        next.authorize(
            &proof,
            &public.peer,
            &[5; 32],
            1,
            ProtectedOperation::Economic
        ),
        Ok(1)
    );
    let stale = VaultSigner::open(&o.vault, &o.game_save_root, o.policy.scope, &local.public)
        .unwrap()
        .sign(&template);
    assert!(stale.is_err());
}
#[test]
fn physical_reopen_selects_real_durably_rotated_device_and_peer_before_fresh_owner_claim() {
    use nf_contract::identity::*;
    use nf_identity::persistence::rotate_device_persisted;
    use nf_store::miniature::{AuthConfig, MiniatureStore};
    use nf_world_driver::{Driver, OpenOptions, SignerOptions};
    let f = driver_support::Fixture::new();
    let o = &f.options;
    let driver = Driver::create(o.clone()).unwrap();
    let initial = driver.status().unwrap();
    drop(driver);
    let vault = PrivateVault::open(&o.vault, &o.game_save_root).unwrap();
    let original = vault.load_identity(o.policy.owner.peer.clone()).unwrap();
    let next_key = SecretSeed::generate().unwrap();
    vault.stage_rotation_key(&next_key).unwrap();
    let change = DeviceRotation {
        scope: o.policy.scope,
        account: original.public.account,
        device: original.public.device,
        frontier: 2,
        new_key: next_key.public_key(),
        new_peer: vec![11, 12],
    };
    let digest = rotation_digest(&change).unwrap();
    let mut store =
        MiniatureStore::open_existing(&o.database, initial.known, AuthConfig::default()).unwrap();
    rotate_device_persisted(
        &mut store,
        o.policy.scope,
        &change,
        &original.account_key.sign(&digest),
        &next_key.sign(&digest),
    )
    .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    drop(next_key);
    let open = OpenOptions {
        database: o.database.clone(),
        scope: o.policy.scope,
        minimum_event: known.storage.event_sequence,
        minimum_store: known.storage.store_revision,
        minimum_membership: 3,
        minimum_term: AuthorityTerm(0),
    };
    let signer = SignerOptions {
        vault: o.vault.clone(),
        game_save_root: o.game_save_root.clone(),
        account: original.public.account,
        device: original.public.device,
    };
    let mut reopened = Driver::open(open, signer)
        .expect("current SQL membership selects matching immutable staged key");
    assert_eq!(reopened.local_identity().device_key, change.new_key);
    assert_eq!(reopened.local_identity().peer, change.new_peer);
    reopened.claim_authority().unwrap();
    assert_eq!(reopened.status().unwrap().world, initial.world);
}
