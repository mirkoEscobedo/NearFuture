mod managed_admission_support;
mod support;
use nf_identity::{
    model::{DeviceRevocation, Invitation, MembershipRepository, Roles},
    rotation::revocation_digest,
    signing::{admission_proof, sign_invitation},
};
use nf_store::{Store, managed::ManagedError};

#[test]
fn ordinary_create_commit_close_reopen_and_signed_adoption_preserves_source() {
    let fixture = managed_admission_support::Fixture::new(1);
    let original = std::fs::read(fixture.scratch.db()).unwrap();
    let owner = fixture.adopt();
    assert_eq!(owner.known_frontiers(), fixture.known);
    assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), original);
    assert!(fixture.catalog().is_file());
}

#[test]
fn exact_32_provider_composition_can_adopt_an_ordinary_reopened_source() {
    let fixture = managed_admission_support::Fixture::new(32);
    let before = std::fs::read(fixture.scratch.db()).unwrap();
    let owner = fixture.adopt();
    assert_eq!(owner.known_frontiers(), fixture.known);
    assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), before);
}

#[test]
fn a_valid_but_different_database_path_cannot_replace_the_actual_open_main_database() {
    let fixture = managed_admission_support::Fixture::new(1);
    let before = std::fs::read(fixture.scratch.db()).unwrap();
    let other = fixture.scratch.0.join("other.sqlite");
    std::fs::copy(fixture.scratch.db(), &other).unwrap();
    let failure = match fixture.begin_at(other.clone(), &fixture.supported) {
        Err(failure) => failure,
        Ok(_) => panic!("different main database path admitted"),
    };
    let (error, source, recovery) = failure.into_parts();
    assert_eq!(error, ManagedError::SourceMismatch);
    assert!(source.is_some());
    assert!(recovery.is_none());
    assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), before);
    assert_eq!(std::fs::read(other).unwrap(), before);
    assert!(!fixture.catalog().exists());
}

#[test]
fn a_durably_revoked_owner_device_cannot_obtain_adoption_authority() {
    let mut fixture = managed_admission_support::Fixture::new(1);
    let mut source = Store::open_existing(fixture.scratch.db(), fixture.known).unwrap();
    let state = source
        .load_membership(fixture.known.scope)
        .unwrap()
        .unwrap();
    let change = DeviceRevocation {
        scope: state.scope,
        issuer: state.owner,
        device: fixture.public.device,
        frontier: state.revision,
    };
    let revoked = state
        .revoke_device(
            &change,
            &fixture.account_key.sign(&revocation_digest(&change)),
        )
        .unwrap();
    source
        .commit_membership(Some(state.revision), &revoked)
        .unwrap();
    fixture.known = source.known_frontiers().unwrap();
    drop(source);
    let before = std::fs::read(fixture.scratch.db()).unwrap();
    let mut draft = fixture.begin(&fixture.supported).unwrap();
    assert!(matches!(
        draft.issue_adoption_challenge(&fixture.public, fixture.cutoff),
        Err(ManagedError::Unauthorized)
    ));
    assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), before);
    assert!(!fixture.catalog().exists());
}

#[test]
fn an_admitted_nonowner_with_all_roles_cannot_obtain_owner_adoption_authority() {
    let mut fixture = managed_admission_support::Fixture::new(1);
    let (other, account, device) =
        nf_identity::keys::generate_identity(b"other-managed-peer".to_vec()).unwrap();
    let mut source = Store::open_existing(fixture.scratch.db(), fixture.known).unwrap();
    let state = source
        .load_membership(fixture.known.scope)
        .unwrap()
        .unwrap();
    let invitation = Invitation {
        scope: state.scope,
        id: [7; 16],
        issuer: state.owner,
        recipient: other.clone(),
        roles: Roles::ALL,
        expires_at: 100,
        issued_revision: state.revision,
        reusable: false,
    };
    let proof = admission_proof(&invitation, &account, &device).unwrap();
    let signed = sign_invitation(invitation, &fixture.account_key).unwrap();
    let next = state.redeem(&signed, &proof, 1).unwrap();
    source
        .commit_membership(Some(state.revision), &next)
        .unwrap();
    fixture.known = source.known_frontiers().unwrap();
    drop(source);
    let before = std::fs::read(fixture.scratch.db()).unwrap();
    let mut draft = fixture.begin(&fixture.supported).unwrap();
    assert!(matches!(
        draft.issue_adoption_challenge(&other, fixture.cutoff),
        Err(ManagedError::Unauthorized)
    ));
    assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), before);
    assert!(!fixture.catalog().exists());
}
