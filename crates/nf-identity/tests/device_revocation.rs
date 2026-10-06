mod support;
use nf_identity::{model::*, rotation::revocation_digest, signing::device_digest};
#[test]
fn revocation_fences_old_device_at_durable_membership_frontier() {
    let c = support::Community::new();
    let invitation = c.invitation();
    let admitted = c
        .state
        .redeem(&invitation, &c.proof(&invitation.invitation), 1)
        .unwrap();
    let mut proof = DeviceProof {
        scope: admitted.scope,
        account: c.recruit.account,
        device: c.recruit.device,
        frontier: 1,
        peer: c.recruit.peer.clone(),
        challenge: [9; 32],
        signature: [0; 64],
    };
    proof.signature = c.device.sign(&device_digest(&proof).unwrap());
    assert_eq!(
        admitted.authorize(
            &proof,
            &proof.peer,
            &proof.challenge,
            1,
            ProtectedOperation::Economic
        ),
        Ok(1)
    );
    assert_eq!(
        admitted.authorize(
            &proof,
            &proof.peer,
            &proof.challenge,
            1,
            ProtectedOperation::Administration
        ),
        Err(IdentityError::RolePolicy)
    );
    let change = DeviceRevocation {
        scope: admitted.scope,
        issuer: c.owner.account,
        device: c.recruit.device,
        frontier: 1,
    };
    let revoked = admitted
        .revoke_device(&change, &c.owner_key.sign(&revocation_digest(&change)))
        .unwrap();
    assert_eq!(revoked.revision, 2);
    assert_eq!(
        revoked.authorize(
            &proof,
            &proof.peer,
            &proof.challenge,
            2,
            ProtectedOperation::Economic
        ),
        Err(IdentityError::Frontier)
    );
    proof.frontier = 2;
    proof.signature = c.device.sign(&device_digest(&proof).unwrap());
    assert_eq!(
        revoked.authorize(
            &proof,
            &proof.peer,
            &proof.challenge,
            2,
            ProtectedOperation::Economic
        ),
        Err(IdentityError::Revoked)
    );
}
