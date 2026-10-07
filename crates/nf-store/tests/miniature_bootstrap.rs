mod miniature_support;
use miniature_support::*;
use nf_identity::model::{DeviceProof, Roles};
use nf_store::{Store, StoreError, miniature::*};
#[test]
fn pinned_public_trust_root_cannot_be_substituted() {
    for case in 0..7 {
        let scratch = Scratch::new();
        let mut f = Fixture::new();
        let mut body = f.bytes();
        match case {
            0 => f.policy.membership_digest[0] ^= 1,
            1 => f.policy.owner.account_key[0] ^= 1,
            2 => f.policy.owner.device_key[0] ^= 1,
            3 => f.policy.owner.peer.push(9),
            4 => f.policy.controllers[2] = f.policy.controllers[1],
            5 => {
                f.membership
                    .accounts
                    .get_mut(&f.policy.owner.account)
                    .unwrap()
                    .roles = Roles::PLAYER;
                body = f.bytes();
                use sha2::{Digest, Sha256};
                f.policy.membership_digest = Sha256::digest(&body).into();
            }
            6 => {
                f.membership
                    .devices
                    .get_mut(&f.policy.owner.device)
                    .unwrap()
                    .revoked = true;
                body = f.bytes();
                use sha2::{Digest, Sha256};
                f.policy.membership_digest = Sha256::digest(&body).into();
            }
            _ => unreachable!(),
        }
        assert!(
            MiniatureStore::create(scratch.db(), f.spec, &body, f.policy, &mut f.signer).is_err(),
            "case {case}"
        );
        assert!(
            !scratch.db().exists(),
            "pin rejection must precede path initialization"
        );
    }
}
struct WrongSigner;
impl BootstrapSigner for WrongSigner {
    fn sign(&mut self, template: &DeviceProof) -> Result<DeviceProof, MiniatureStoreError> {
        let key = nf_identity::keys::SecretSeed::generate()?;
        let mut p = template.clone();
        p.signature = key.sign(&nf_identity::signing::device_digest(&p)?);
        Ok(p)
    }
}
#[test]
fn supplied_public_owner_without_matching_secret_cannot_initialize() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    assert!(
        MiniatureStore::create(scratch.db(), f.spec, &f.bytes(), f.policy, &mut WrongSigner)
            .is_err()
    );
    assert!(!scratch.db().exists());
}
#[test]
fn profile1_reader_rejects_miniature_without_changing_any_file_bytes() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let bytes = std::fs::read(scratch.db()).unwrap();
    assert!(matches!(
        Store::open_existing(scratch.db(), known.storage),
        Err(StoreError::UnsupportedSchema)
    ));
    assert_eq!(std::fs::read(scratch.db()).unwrap(), bytes);
}
