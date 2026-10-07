use super::{Signer, membership, vector};
use nf_kernel::miniature::*;
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
pub fn create(path: &std::path::Path) -> (MiniatureStore, Signer) {
    let (state, owner, mut signer) = membership();
    let body = nf_identity::codec::encode_state(&state).unwrap();
    let bytes = vector("envelope", "genesis");
    let world = decode_miniature_snapshot(&bytes[49..1260]).unwrap();
    let metadata = world.metadata();
    let spec = MiniatureGenesisSpec {
        genesis: world.component().genesis(),
        aggregate: metadata.aggregate,
        provider: metadata.provider,
        provider_aggregate: metadata.provider_aggregate,
    };
    let policy = BootstrapPolicy {
        scope: state.scope,
        membership_digest: Sha256::digest(&body).into(),
        owner,
        controllers: spec.genesis.accounts,
        auth: AuthConfig::default(),
    };
    let store = MiniatureStore::create(path, spec, &body, policy, &mut signer).unwrap();
    (store, signer)
}
