//! Pure fixture evidence and real key proofs; no authority constructor or mock witness.
mod cursor;
mod fixture;
mod records;
mod observations;
mod schemas;
mod world;
mod transcripts;
mod catalog;
mod catalog_replay;
mod operations;
mod retirement;
pub use fixture::{Case,bounded_read,now_ms};
pub use records::{PROFILE1_SCHEMA,PROFILE4_SCHEMA,hash,composition,composition_bytes,descriptor,
    completed_core,prepared_core,envelope,source_binding,upgrade_action,lineage,journal_head,
    prepare_payload,settlement_payload,request_binding,PrepareTimes};
pub use observations::{base_rows,source_head,source_foot,rows_hash};
pub use world::WorldOracle;
pub use transcripts::{Audit,ProspectiveExpected,ExpectedPre,prospective,owned,principal,assert_epoch,action as authenticated_action};
pub use catalog::{admission_delta,verify_manifests};
pub use catalog_replay::{Expected,Operation,replay as replay_catalog};
pub use operations::{quiesce_payload,registration_payload,registration_delta,artifact_pins,
    assert_adoption_operation,assert_claim_operation,assert_selection_operation,physical_effect,DATA_PROVENANCE};
pub use retirement::{record as retirement_record,foot as retired_foot,rows as retired_rows,observe as observe_retired};
use nf_identity::{keys::SecretSeed,model::{DeviceProof,MembershipState,ProtectedOperation,PublicIdentity,Scope},
    signing::device_digest};
pub fn catalog_tuple(frontier:&nf_store::managed::CatalogFrontier)->(u64,[u8;32],[u8;32]){
    (frontier.revision(),frontier.state_digest(),frontier.head())
}
pub fn sign(public:&PublicIdentity,key:&SecretSeed,scope:Scope,frontier:u64,challenge:[u8;32])->DeviceProof{
    let mut proof=DeviceProof{scope,account:public.account,device:public.device,frontier,
        peer:public.peer.clone(),challenge,signature:[0;64]};
    proof.signature=key.sign(&device_digest(&proof).unwrap());proof
}
pub fn authorize(member:&MembershipState,proof:&DeviceProof){
    assert_eq!(member.authorize(proof,&proof.peer,&proof.challenge,member.revision,
        ProtectedOperation::Administration).unwrap(),member.revision);
}
pub fn settled(world:&nf_kernel::World,frontier:&nf_kernel::Frontier,results:Vec<nf_kernel::JobResult>,
    context:nf_kernel::AuthorityContext)->(nf_kernel::World,nf_kernel::CommittedBatch){
    match nf_kernel::settle(world,frontier,results,context,nf_kernel::MissingPolicy::Pause).unwrap(){
        nf_kernel::Settlement::Committed{world,batch}=>(*world,batch),
        nf_kernel::Settlement::Paused(_)=>panic!("genuine complete worker proposal did not settle"),
    }
}
