mod driver_support;
use nf_contract::identity::*;
use nf_identity::{model::*, private_storage::PrivateVault};
use nf_world_driver::VaultSigner;
#[test]
fn existing_private_vault_signs_exact_public_device_template_with_real_membership_verification() {
    let scratch = driver_support::Scratch::new();
    let path = scratch.0.join("private");
    let saves = scratch.0.join("saves");
    let vault = PrivateVault::create(&path, &saves).unwrap();
    let local = vault.create_identity(vec![1, 2]).unwrap();
    let public = local.public.clone();
    drop(local);
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let membership = MembershipState::bootstrap(scope, &public).unwrap();
    let signer =
        VaultSigner::open(&path, &saves, scope, &public).expect("load existing matching keys");
    let template = DeviceProof {
        scope,
        account: public.account,
        device: public.device,
        frontier: 0,
        peer: public.peer.clone(),
        challenge: [5; 32],
        signature: [0; 64],
    };
    let proof = signer.sign(&template).unwrap();
    assert_eq!(
        membership.authorize(
            &proof,
            &public.peer,
            &[5; 32],
            0,
            ProtectedOperation::Economic
        ),
        Ok(0)
    );
    let mut altered = proof;
    altered.challenge[0] ^= 1;
    assert_eq!(
        membership.authorize(
            &altered,
            &public.peer,
            &altered.challenge,
            0,
            ProtectedOperation::Economic
        ),
        Err(IdentityError::Signature)
    );
    assert_eq!(std::fs::read_dir(&saves).unwrap().count(), 0);
}
#[test]
fn mismatched_scope_device_peer_and_existing_signature_never_reach_private_signing() {
    let scratch = driver_support::Scratch::new();
    let path = scratch.0.join("private");
    let saves = scratch.0.join("saves");
    let vault = PrivateVault::create(&path, &saves).unwrap();
    let local = vault.create_identity(vec![1, 2]).unwrap();
    let public = local.public.clone();
    drop(local);
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let mut signer = VaultSigner::open(&path, &saves, scope, &public).unwrap();
    let template = DeviceProof {
        scope,
        account: public.account,
        device: public.device,
        frontier: 0,
        peer: public.peer.clone(),
        challenge: [5; 32],
        signature: [0; 64],
    };
    assert!(nf_store::miniature::BootstrapSigner::sign(&mut signer, &template).is_ok());
    let mut wrong = template.clone();
    wrong.scope.history = HistoryId::from_bytes([9; 16]);
    assert_eq!(
        signer.sign(&wrong),
        Err(nf_world_driver::SignerError::WrongTemplate)
    );
    wrong = template.clone();
    wrong.device = DeviceId::from_bytes([9; 16]);
    assert!(signer.sign(&wrong).is_err());
    wrong = template.clone();
    wrong.peer = vec![9];
    assert!(signer.sign(&wrong).is_err());
    wrong = template;
    wrong.signature = [1; 64];
    assert!(signer.sign(&wrong).is_err());
    let mut mismatched = public.clone();
    mismatched.device_key = [9; 32];
    assert!(VaultSigner::open(&path, &saves, scope, &mismatched).is_err());
    std::fs::remove_file(path.join("identity-key-v1")).unwrap();
    assert!(VaultSigner::open(&path, &saves, scope, &public).is_err());
    assert!(!path.join("identity-key-v1").exists());
}
#[test]
fn peer_input_is_bounded_before_private_file_effects_or_cloning() {
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let mut expected = PublicIdentity {
        account: AccountId::from_bytes([3; 16]),
        account_key: [4; 32],
        device: DeviceId::from_bytes([5; 16]),
        device_key: [6; 32],
        peer: vec![7; 129],
    };
    for peer in [vec![7; 129], vec![]] {
        expected.peer = peer;
        assert_eq!(
            VaultSigner::open(
                std::path::Path::new("absent-private-vault"),
                std::path::Path::new("absent-save-root"),
                scope,
                &expected
            )
            .err(),
            Some(nf_world_driver::SignerError::WrongIdentity)
        );
    }
}
