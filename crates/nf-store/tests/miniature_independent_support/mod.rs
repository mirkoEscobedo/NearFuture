use nf_contract::identity::*;
use nf_identity::{keys::SecretSeed, model::*};
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
pub fn hex(value: &str) -> Vec<u8> {
    assert!(value.len() <= 2_097_152 && value.len().is_multiple_of(2));
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub fn vector(category: &str, name: &str) -> Vec<u8> {
    let text = include_str!("../../../../docs/world/vectors/store-v2.tsv");
    assert!(text.len() <= 262_144);
    let row = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 6);
            (fields[0] == category && fields[1] == name).then_some(fields)
        })
        .unwrap();
    let bytes = hex(row[3]);
    assert_eq!(bytes.len(), row[2].parse::<usize>().unwrap());
    assert_eq!(Sha256::digest(&bytes).as_slice(), hex(row[4]));
    bytes
}
pub struct Signer(pub SecretSeed);
impl BootstrapSigner for Signer {
    fn sign(&mut self, template: &DeviceProof) -> Result<DeviceProof, MiniatureStoreError> {
        let mut proof = template.clone();
        proof.signature = self.0.sign(&nf_identity::signing::device_digest(&proof)?);
        Ok(proof)
    }
}
pub fn membership() -> (MembershipState, PublicIdentity, Signer) {
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let (mut owner, _, device_key) = nf_identity::keys::generate_identity(vec![1, 2, 3]).unwrap();
    owner.account = AccountId::from_bytes([4; 16]);
    owner.device = DeviceId::from_bytes([33; 16]);
    let mut state = MembershipState::bootstrap(scope, &owner).unwrap();
    for ordinal in [5, 6] {
        let (public, _, _) = nf_identity::keys::generate_identity(vec![ordinal]).unwrap();
        state.accounts.insert(
            AccountId::from_bytes([ordinal; 16]),
            Account {
                key: public.account_key,
                roles: Roles::PLAYER,
            },
        );
    }
    nf_identity::codec::validate_state(&state).unwrap();
    (state, owner, Signer(device_key))
}

mod scratch;
pub use scratch::Scratch;
pub mod admission;
pub mod challenge;
pub mod fixture;
pub mod schema;
