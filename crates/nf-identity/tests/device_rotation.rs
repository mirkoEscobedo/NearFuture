mod support;
use nf_identity::{keys::SecretSeed, model::*, rotation::rotation_digest, signing::device_digest};
#[test]
fn authenticated_device_rotation_changes_key_and_transport_binding_without_role_escalation() {
    let c = support::Community::new();
    let invitation = c.invitation();
    let admitted = c
        .state
        .redeem(&invitation, &c.proof(&invitation.invitation), 1)
        .unwrap();
    let new_key = SecretSeed::generate().unwrap();
    let change = DeviceRotation {
        scope: admitted.scope,
        account: c.recruit.account,
        device: c.recruit.device,
        frontier: 1,
        new_key: new_key.public_key(),
        new_peer: vec![5, 6],
    };
    let digest = rotation_digest(&change).unwrap();
    let rotated = admitted
        .rotate_device(&change, &c.account.sign(&digest), &new_key.sign(&digest))
        .unwrap();
    assert_eq!(rotated.revision, 2);
    assert_eq!(rotated.accounts[&c.recruit.account].roles, Roles::PLAYER);
    let mut proof = DeviceProof {
        scope: rotated.scope,
        account: c.recruit.account,
        device: c.recruit.device,
        frontier: 2,
        peer: vec![5, 6],
        challenge: [7; 32],
        signature: [0; 64],
    };
    proof.signature = new_key.sign(&device_digest(&proof).unwrap());
    assert_eq!(
        rotated.authorize(&proof, &[5, 6], &[7; 32], 2, ProtectedOperation::Economic),
        Ok(2)
    );
    assert_eq!(
        rotated.authorize(&proof, &[3, 4], &[7; 32], 2, ProtectedOperation::Economic),
        Err(IdentityError::Signature)
    );
    assert_eq!(
        rotated.authorize(&proof, &[5, 6], &[8; 32], 2, ProtectedOperation::Economic),
        Err(IdentityError::Signature)
    );
    proof.signature = c.device.sign(&device_digest(&proof).unwrap());
    assert_eq!(
        rotated.authorize(&proof, &[5, 6], &[7; 32], 2, ProtectedOperation::Economic),
        Err(IdentityError::Signature)
    );
    assert_eq!(
        admitted.rotate_device(&change, &c.device.sign(&digest), &new_key.sign(&digest)),
        Err(IdentityError::Signature)
    );
}
