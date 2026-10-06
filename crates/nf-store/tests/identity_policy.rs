mod support;
use nf_identity::{
    keys::{SecretSeed, generate_identity, random_id},
    model::*,
    persistence::{redeem_persisted, revoke_persisted, rotate_device_persisted},
    private_storage::PrivateVault,
    rotation::{revocation_digest, rotation_digest},
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::{Store, StoreError};

#[test]
fn signed_admission_revocation_and_private_key_rotation_survive_real_sqlite_restarts() {
    let scratch = support::Scratch::new();
    let world = support::world();
    let mut store = Store::create(scratch.db(), &world).unwrap();
    let scope = store.known_frontiers().unwrap().scope;
    let (owner, owner_key, _) = generate_identity(vec![1, 2]).unwrap();
    let initial = MembershipState::bootstrap(scope, &owner).unwrap();
    store.commit_membership(None, &initial).unwrap();
    let saves = scratch.0.join("saves");
    std::fs::create_dir(&saves).unwrap();
    let private = scratch.0.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    let recipient = vault.create_identity(vec![3, 4]).unwrap();
    let invitation = sign_invitation(
        Invitation {
            scope,
            id: random_id().unwrap(),
            issuer: owner.account,
            recipient: recipient.public.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        },
        &owner_key,
    )
    .unwrap();
    let proof = admission_proof(
        &invitation.invitation,
        &recipient.account_key,
        &recipient.device_key,
    )
    .unwrap();
    let admitted = redeem_persisted(&mut store, scope, &invitation, &proof, 1).unwrap();
    assert_eq!(admitted.revision, 1);
    let backup = scratch.0.join("pre-revocation.sqlite");
    store.backup_to(&backup).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(
        redeem_persisted(&mut store, scope, &invitation, &proof, 1),
        Err(IdentityError::Replay)
    );
    let mut stale = initial.clone();
    stale.revision = 1;
    assert_eq!(
        store.commit_membership(Some(0), &stale),
        Err(IdentityError::Conflict)
    );
    let revoke = DeviceRevocation {
        scope,
        issuer: owner.account,
        device: recipient.public.device,
        frontier: 1,
    };
    revoke_persisted(
        &mut store,
        scope,
        &revoke,
        &owner_key.sign(&revocation_digest(&revoke)),
    )
    .unwrap();
    let revoked_frontier = store.known_frontiers().unwrap();
    drop(store);
    assert!(matches!(
        Store::open_existing(backup, revoked_frontier),
        Err(StoreError::StaleBackup)
    ));
    let mut store = Store::open_existing(scratch.db(), revoked_frontier).unwrap();
    let revoked = store.load_membership(scope).unwrap().unwrap();
    let mut device_proof = DeviceProof {
        scope,
        account: recipient.public.account,
        device: recipient.public.device,
        frontier: 2,
        peer: vec![3, 4],
        challenge: [5; 32],
        signature: [0; 64],
    };
    device_proof.signature = recipient
        .device_key
        .sign(&device_digest(&device_proof).unwrap());
    assert_eq!(
        revoked.authorize(
            &device_proof,
            &[3, 4],
            &[5; 32],
            2,
            ProtectedOperation::Economic
        ),
        Err(IdentityError::Revoked)
    );
    let new_key = SecretSeed::generate().unwrap();
    vault.stage_rotation_key(&new_key).unwrap();
    let change = DeviceRotation {
        scope,
        account: recipient.public.account,
        device: recipient.public.device,
        frontier: 2,
        new_key: new_key.public_key(),
        new_peer: vec![6, 7],
    };
    let digest = rotation_digest(&change).unwrap();
    rotate_device_persisted(
        &mut store,
        scope,
        &change,
        &recipient.account_key.sign(&digest),
        &new_key.sign(&digest),
    )
    .unwrap();
    let rotated_frontier = store.known_frontiers().unwrap();
    drop(store);
    drop(new_key);
    let mut store = Store::open_existing(scratch.db(), rotated_frontier).unwrap();
    let rotated = store.load_membership(scope).unwrap().unwrap();
    let mut selected = recipient.public.clone();
    selected.device_key = rotated.devices[&selected.device].key;
    selected.peer = rotated.devices[&selected.device].peer.clone();
    let restarted_keys = PrivateVault::open(&private, &saves)
        .unwrap()
        .load_membership_keys(&selected)
        .unwrap();
    device_proof.frontier = 3;
    device_proof.peer = selected.peer;
    device_proof.signature = restarted_keys
        .device_key
        .sign(&device_digest(&device_proof).unwrap());
    assert_eq!(
        rotated.authorize(
            &device_proof,
            &[6, 7],
            &[5; 32],
            3,
            ProtectedOperation::Economic
        ),
        Ok(3)
    );
    assert_eq!(
        rotated.authorize(
            &device_proof,
            &[6, 7],
            &[5; 32],
            3,
            ProtectedOperation::Administration
        ),
        Err(IdentityError::RolePolicy)
    );
    device_proof.signature = recipient
        .device_key
        .sign(&device_digest(&device_proof).unwrap());
    assert_eq!(
        rotated.authorize(
            &device_proof,
            &[6, 7],
            &[5; 32],
            3,
            ProtectedOperation::Economic
        ),
        Err(IdentityError::Signature)
    );
    assert!(rotated.consumed.contains(&invitation.invitation.id));
}
