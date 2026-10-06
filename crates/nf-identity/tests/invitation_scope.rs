use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::{generate_identity, random_id},
    model::*,
};

#[test]
fn history_pinning_prevents_save_or_invitation_copy_admission() {
    let (founder, account, _) = generate_identity(vec![1]).unwrap();
    let (recipient, _, _) = generate_identity(vec![2]).unwrap();
    let scope = Scope {
        universe: UniverseId::from_bytes(random_id().unwrap()),
        history: HistoryId::from_bytes(random_id().unwrap()),
    };
    let state = MembershipState::bootstrap(scope, &founder).unwrap();
    let invitation = Invitation {
        scope: Scope {
            history: HistoryId::from_bytes(random_id().unwrap()),
            ..scope
        },
        id: random_id().unwrap(),
        issuer: founder.account,
        recipient,
        roles: Roles::PLAYER,
        expires_at: 100,
        issued_revision: 0,
        reusable: false,
    };
    let signed = SignedInvitation {
        invitation,
        signature: account.sign(&[0; 32]),
    };
    let proof = AdmissionProof {
        account_signature: [0; 64],
        device_signature: [0; 64],
    };
    assert_eq!(state.redeem(&signed, &proof, 1), Err(IdentityError::Scope));
}
