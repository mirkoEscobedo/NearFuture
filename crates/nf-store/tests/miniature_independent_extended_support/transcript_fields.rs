use super::{Signer, attempt, hex, vector};
use nf_kernel::miniature::encode_miniature_frontier;
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
pub fn compare(
    issued: &IssuedChallenge,
    name: &str,
    store: &MiniatureStore,
    sequence: u64,
    membership: u64,
    binding: [u8; 32],
) {
    let actual = issued.challenge_preimage();
    let mut expected = vector("auth", name);
    let a = store.authority().unwrap();
    for (offset, value) in [
        (80, a.term.0),
        (88, a.term.0),
        (96, a.session.0),
        (112, membership),
        (184, sequence),
    ] {
        expected[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    expected[152..184].copy_from_slice(&binding);
    expected[192..224].copy_from_slice(&actual[192..224]);
    assert_eq!(actual.as_slice(), expected);
    assert_eq!(
        issued.template.challenge,
        <[u8; 32]>::from(Sha256::digest(actual))
    );
}
pub fn frontier(store: &MiniatureStore, membership: u64) -> Vec<u8> {
    let mut bytes = hex(include_str!(
        "../../../../crates/nf-kernel/tests/fixtures/miniature/frontier-colony.hex"
    )
    .trim());
    let a = store.authority().unwrap();
    for (offset, value) in [(49, a.term.0), (57, a.session.0), (73, membership)] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    assert_eq!(
        bytes,
        encode_miniature_frontier(store.pending().unwrap()).unwrap()
    );
    bytes
}
pub fn mismatch_cancel(store: &mut MiniatureStore, signer: &mut Signer, issued: &IssuedChallenge) {
    let proof = attempt(issued, signer);
    let retry = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    assert_eq!(
        store.cancel_pending(MiniatureCancelCause::Cancelled, proof),
        Err(MiniatureStoreError::WrongPurpose)
    );
    assert_eq!(
        store.cancel_pending(MiniatureCancelCause::Cancelled, retry),
        Err(MiniatureStoreError::Replay)
    );
}
