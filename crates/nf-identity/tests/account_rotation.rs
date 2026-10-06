mod support;
use nf_identity::{
    keys::SecretSeed,
    model::*,
    rotation::{account_rotation_digest, rotation_digest},
};
#[test]
fn account_rotation_requires_old_and_new_key_possession_and_keeps_account_continuity() {
    let c = support::Community::new();
    let invite = c.invitation();
    let admitted = c
        .state
        .redeem(&invite, &c.proof(&invite.invitation), 1)
        .unwrap();
    let new_account = SecretSeed::generate().unwrap();
    let change = AccountRotation {
        scope: admitted.scope,
        account: c.recruit.account,
        frontier: 1,
        new_key: new_account.public_key(),
    };
    let digest = account_rotation_digest(&change);
    let rotated = admitted
        .rotate_account(
            &change,
            &c.account.sign(&digest),
            &new_account.sign(&digest),
        )
        .unwrap();
    assert_eq!(
        rotated.accounts[&c.recruit.account].key,
        new_account.public_key()
    );
    assert_eq!(rotated.accounts[&c.recruit.account].roles, Roles::PLAYER);
    assert_eq!(rotated.owner, c.owner.account);
    let replacement = SecretSeed::generate().unwrap();
    let device_change = DeviceRotation {
        scope: rotated.scope,
        account: c.recruit.account,
        device: c.recruit.device,
        frontier: 2,
        new_key: replacement.public_key(),
        new_peer: vec![8],
    };
    let digest = rotation_digest(&device_change).unwrap();
    assert_eq!(
        rotated.rotate_device(
            &device_change,
            &c.account.sign(&digest),
            &replacement.sign(&digest)
        ),
        Err(IdentityError::Signature)
    );
    assert!(
        rotated
            .rotate_device(
                &device_change,
                &new_account.sign(&digest),
                &replacement.sign(&digest)
            )
            .is_ok()
    );
    assert_eq!(
        admitted.rotate_account(
            &change,
            &c.device.sign(&account_rotation_digest(&change)),
            &new_account.sign(&account_rotation_digest(&change))
        ),
        Err(IdentityError::Signature)
    );
}
