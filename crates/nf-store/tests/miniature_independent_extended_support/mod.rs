#[path = "../miniature_independent_support/mod.rs"]
#[allow(
    dead_code,
    reason = "reuse frozen support independently exercised by its original test target"
)]
mod baseline;
pub use baseline::fixture::create;
pub use baseline::{Scratch, Signer, hex, vector};
pub mod lifetime;
pub mod minima;
pub mod mirrors;
pub mod transcripts;
use nf_contract::identity::*;
use nf_store::miniature::*;
pub fn claim(store: &mut MiniatureStore, signer: &mut Signer) {
    let issued = store
        .issue_challenge(ChallengeRequest::Claim {
            actor: AccountId::from_bytes([4; 16]),
            device: DeviceId::from_bytes([33; 16]),
        })
        .unwrap();
    let proof = signer.sign(&issued.template).unwrap();
    store
        .claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
        .unwrap();
}
pub fn attempt(issued: &IssuedChallenge, signer: &mut Signer) -> ProofAttempt {
    ProofAttempt {
        ticket: issued.ticket,
        proof: signer.sign(&issued.template).unwrap(),
    }
}
pub mod transcript_fields;
