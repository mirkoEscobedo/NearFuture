//! Pure fixture evidence and real key proofs; no authority constructor or mock witness.
mod catalog;
mod catalog_replay;
mod cursor;
mod fixture;
mod observations;
mod operations;
mod records;
mod retirement;
mod schemas;
mod transcripts;
mod world;
pub use catalog::{admission_delta, verify_manifests};
pub use catalog_replay::{Expected, Operation, replay as replay_catalog};
pub use fixture::{Case, bounded_read, now_ms};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity, Scope},
    signing::device_digest,
};
pub use observations::{base_rows, rows_hash, source_foot, source_head};
pub use operations::{
    DATA_PROVENANCE, artifact_pins, assert_adoption_operation, assert_claim_operation,
    assert_selection_operation, physical_effect, quiesce_payload, registration_delta,
    registration_payload,
};
pub use records::{
    PROFILE1_SCHEMA, PROFILE4_SCHEMA, PrepareTimes, completed_core, composition, composition_bytes,
    descriptor, envelope, hash, journal_head, lineage, prepare_payload, prepared_core,
    request_binding, settlement_payload, source_binding, upgrade_action,
};
pub use retirement::{
    foot as retired_foot, observe as observe_retired, record as retirement_record,
    rows as retired_rows,
};
pub use transcripts::{
    Audit, ExpectedPre, ProspectiveExpected, action as authenticated_action, assert_epoch, owned,
    principal, prospective,
};
pub use world::WorldOracle;
pub fn catalog_tuple(frontier: &nf_store::managed::CatalogFrontier) -> (u64, [u8; 32], [u8; 32]) {
    (
        frontier.revision(),
        frontier.state_digest(),
        frontier.head(),
    )
}
pub fn sign(
    public: &PublicIdentity,
    key: &SecretSeed,
    scope: Scope,
    frontier: u64,
    challenge: [u8; 32],
) -> DeviceProof {
    let mut proof = DeviceProof {
        scope,
        account: public.account,
        device: public.device,
        frontier,
        peer: public.peer.clone(),
        challenge,
        signature: [0; 64],
    };
    proof.signature = key.sign(&device_digest(&proof).unwrap());
    proof
}
pub fn authorize(member: &MembershipState, proof: &DeviceProof) {
    assert_eq!(
        member
            .authorize(
                proof,
                &proof.peer,
                &proof.challenge,
                member.revision,
                ProtectedOperation::Administration
            )
            .unwrap(),
        member.revision
    );
}
pub fn settled(
    world: &nf_kernel::World,
    frontier: &nf_kernel::Frontier,
    results: Vec<nf_kernel::JobResult>,
    context: nf_kernel::AuthorityContext,
) -> (nf_kernel::World, nf_kernel::CommittedBatch) {
    match nf_kernel::settle(
        world,
        frontier,
        results,
        context,
        nf_kernel::MissingPolicy::Pause,
    )
    .unwrap()
    {
        nf_kernel::Settlement::Committed { world, batch } => (*world, batch),
        nf_kernel::Settlement::Paused(_) => {
            panic!("genuine complete worker proposal did not settle")
        }
    }
}
