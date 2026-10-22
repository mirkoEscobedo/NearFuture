//! One copied-fact diagnostic transaction; Java retains all game decision authority.
//! These are requested reference facets, not invented AI scheduling or native world certification.
use crate::{
    BindingScope, MakePeaceEligibility, PeaceFacts, PeaceResult, ShadowAuthority, ShadowMetadata,
    Unavailable, WarFacts, WarOperation, WarResult,
};
use nf_nex_boundary::Modifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstProposalInput {
    pub concern_operation: WarOperation,
    pub concern: WarFacts,
    pub eligibility: MakePeaceEligibility,
    pub selected: PeaceFacts,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstProposalOutput {
    pub concern: WarResult,
    pub eligible: bool,
    pub additional_priority: Modifier,
    pub selected: PeaceResult,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstProposalEvaluation {
    pub(crate) metadata: ShadowMetadata,
    pub(crate) binding_scope: BindingScope,
    pub(crate) input_digest: [u8; 32],
    pub(crate) output: FirstProposalOutput,
}
impl FirstProposalEvaluation {
    pub fn metadata(&self) -> &ShadowMetadata {
        &self.metadata
    }
    pub fn binding_scope(&self) -> BindingScope {
        self.binding_scope
    }
    pub fn input_digest(&self) -> [u8; 32] {
        self.input_digest
    }
    pub fn output(&self) -> &FirstProposalOutput {
        &self.output
    }
    pub fn authority(&self) -> ShadowAuthority {
        ShadowAuthority::ShadowOnly
    }
}
pub(crate) fn evaluate_first_proposal(
    metadata: &ShadowMetadata,
    input: &FirstProposalInput,
) -> Result<FirstProposalEvaluation, Unavailable> {
    let digest = crate::first_proposal_input_digest(metadata, input)?;
    // Repeated source facts must be consistent; supplied facets remain immutable.
    if input.concern.weariness != input.selected.own_weariness
        || input.concern.minimum != input.selected.rules.minimum
    {
        return Err(Unavailable::Stale);
    }
    let output = FirstProposalOutput {
        concern: crate::evaluate_war(&input.concern, input.concern_operation)?,
        eligible: crate::make_peace_action_eligible(&input.eligibility)?,
        additional_priority: crate::make_peace_weariness_modifier(
            input.concern.weariness,
            input.concern.minimum,
        )?,
        selected: crate::evaluate_selected_peace(&input.selected)?,
    };
    Ok(FirstProposalEvaluation {
        metadata: metadata.clone(),
        binding_scope: BindingScope::CopiedFacts,
        input_digest: digest,
        output,
    })
}
