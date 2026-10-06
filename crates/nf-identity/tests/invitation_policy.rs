mod support;
use nf_identity::{model::*, signing::sign_invitation};
#[test]
fn expiry_signature_recipient_and_role_policies_fail_closed() {
    let c = support::Community::new();
    let invite = c.invitation();
    let proof = c.proof(&invite.invitation);
    assert_eq!(
        c.state.redeem(&invite, &proof, 100),
        Err(IdentityError::Expired)
    );
    let mut altered = invite.clone();
    altered.invitation.roles = Roles::ALL;
    assert_eq!(
        c.state.redeem(&altered, &proof, 1),
        Err(IdentityError::Signature)
    );
    let mut bad_proof = proof.clone();
    bad_proof.device_signature[0] ^= 1;
    assert_eq!(
        c.state.redeem(&invite, &bad_proof, 1),
        Err(IdentityError::Signature)
    );
    let mut changed = invite.invitation.clone();
    changed.reusable = true;
    let reusable = sign_invitation(changed, &c.owner_key).unwrap();
    assert_eq!(
        c.state.redeem(&reusable, &c.proof(&reusable.invitation), 1),
        Err(IdentityError::RolePolicy)
    );
    let mut changed = invite.invitation;
    changed.issuer = c.recruit.account;
    let unadmitted = sign_invitation(changed, &c.account).unwrap();
    assert_eq!(
        c.state
            .redeem(&unadmitted, &c.proof(&unadmitted.invitation), 1),
        Err(IdentityError::UnknownAccount)
    );
}
