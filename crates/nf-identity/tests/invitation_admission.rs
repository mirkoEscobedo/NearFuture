mod support;
use nf_identity::model::*;
#[test]
fn signed_one_use_invitation_admits_exact_recipient_and_roles() {
    let community = support::Community::new();
    let invite = community.invitation();
    let proof = community.proof(&invite.invitation);
    let admitted = community.state.redeem(&invite, &proof, 99).unwrap();
    assert_eq!(
        admitted.accounts[&community.recruit.account].roles,
        Roles::PLAYER
    );
    assert_eq!(
        admitted.devices[&community.recruit.device].account,
        community.recruit.account
    );
    assert_eq!(admitted.revision, 1);
    assert_eq!(
        admitted.redeem(&invite, &proof, 99),
        Err(IdentityError::Replay)
    );
}
