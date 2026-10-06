mod support;
use nf_identity::{keys::generate_identity, model::*, signing::sign_invitation};
#[test]
fn moderator_and_compute_roles_do_not_imply_owner_or_economic_authority() {
    let c = support::Community::new();
    let mut invitation = c.invitation().invitation;
    invitation.roles = Roles::MODERATOR;
    let signed = sign_invitation(invitation, &c.owner_key).unwrap();
    let moderated = c
        .state
        .redeem(&signed, &c.proof(&signed.invitation), 1)
        .unwrap();
    let (target, account, device) = generate_identity(vec![8]).unwrap();
    let invitation = Invitation {
        recipient: target,
        issuer: c.recruit.account,
        roles: Roles::ALL,
        issued_revision: 1,
        id: nf_identity::keys::random_id().unwrap(),
        ..signed.invitation
    };
    let elevated = sign_invitation(invitation, &c.account).unwrap();
    let proof =
        nf_identity::signing::admission_proof(&elevated.invitation, &account, &device).unwrap();
    assert_eq!(
        moderated.redeem(&elevated, &proof, 1),
        Err(IdentityError::RolePolicy)
    );
    assert_eq!(
        moderated.recover_lost_account(),
        Err(IdentityError::LostKeyRecoveryRefused)
    );
    assert!(!Roles::WORKER.contains(Roles::PLAYER));
    assert!(!Roles::WORKER.contains(Roles::AUTHORITY_CANDIDATE));
    assert!(!Roles::AUTHORITY_CANDIDATE.contains(Roles::MODERATOR));
}
