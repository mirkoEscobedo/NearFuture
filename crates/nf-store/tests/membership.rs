mod support;
use nf_identity::{
    codec::encode_state,
    model::{IdentityError, MembershipRepository, MembershipState, Scope},
};
use nf_store::{Store, StoreError};
fn membership(scope: Scope) -> MembershipState {
    let (public, _account, _device) = nf_identity::keys::generate_identity(vec![1, 2, 3]).unwrap();
    MembershipState::bootstrap(scope, &public).unwrap()
}
#[test]
fn public_membership_cas_restart_and_backup_frontier_prevent_revocation_rollback() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let mut store = Store::create(scratch.db(), &world).unwrap();
    let scope = store.known_frontiers().unwrap().scope;
    let initial = membership(scope);
    store.commit_membership(None, &initial).unwrap();
    assert_eq!(
        store.commit_membership(None, &initial),
        Err(IdentityError::Conflict)
    );
    let backup = scratch.0.join("old.sqlite");
    store.backup_to(&backup).unwrap();
    let mut revoked = initial.clone();
    revoked.revision = 1;
    revoked.devices.values_mut().next().unwrap().revoked = true;
    revoked.consumed.insert([9; 16]);
    store.commit_membership(Some(0), &revoked).unwrap();
    let mut stale = initial.clone();
    stale.revision = 1;
    assert_eq!(
        store.commit_membership(Some(0), &stale),
        Err(IdentityError::Conflict)
    );
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = Store::open_existing(scratch.db(), known).unwrap();
    let restored = store.load_membership(scope).unwrap().unwrap();
    assert_eq!(
        encode_state(&restored).unwrap(),
        encode_state(&revoked).unwrap()
    );
    assert!(restored.consumed.contains(&[9; 16]));
    assert!(matches!(
        Store::open_existing(backup, known),
        Err(StoreError::StaleBackup)
    ));
}
