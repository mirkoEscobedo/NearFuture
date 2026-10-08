use crate::{PeaceResult, ShadowInput, ShadowMetadata, Unavailable, WarResult, input_digest};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShadowOutput {
    War(WarResult),
    SelectedPeace(PeaceResult),
    MakePeaceEligibility(bool),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowAuthority {
    ShadowOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingScope {
    CopiedFacts,
    WorldWithoutCommitment,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowEvaluation {
    metadata: ShadowMetadata,
    binding_scope: BindingScope,
    input_digest: [u8; 32],
    output: ShadowOutput,
}
impl ShadowEvaluation {
    pub fn binding_scope(&self) -> BindingScope {
        self.binding_scope
    }
    pub(crate) fn require_world_commitment(mut self) -> Self {
        self.binding_scope = BindingScope::WorldWithoutCommitment;
        self
    }
    pub fn metadata(&self) -> &ShadowMetadata {
        &self.metadata
    }
    pub fn input_digest(&self) -> [u8; 32] {
        self.input_digest
    }
    pub fn output(&self) -> &ShadowOutput {
        &self.output
    }
    pub fn authority(&self) -> ShadowAuthority {
        ShadowAuthority::ShadowOnly
    }
}
/// Diagnostic evaluation over explicitly copied supported facts; not a world publication certificate.
pub fn evaluate_shadow(
    metadata: &ShadowMetadata,
    input: &ShadowInput,
) -> Result<ShadowEvaluation, Unavailable> {
    let digest = input_digest(metadata, input)?;
    let output = match input {
        ShadowInput::War { operation, facts } => {
            ShadowOutput::War(crate::evaluate_war(facts, *operation)?)
        }
        ShadowInput::SelectedPeace(facts) => {
            ShadowOutput::SelectedPeace(crate::evaluate_selected_peace(facts)?)
        }
    };
    Ok(ShadowEvaluation {
        metadata: metadata.clone(),
        binding_scope: BindingScope::CopiedFacts,
        input_digest: digest,
        output,
    })
}
fn evaluate_action_shadow(
    metadata: &ShadowMetadata,
    facts: &crate::MakePeaceEligibility,
) -> Result<ShadowEvaluation, Unavailable> {
    let digest = crate::action_input_digest(metadata, facts)?;
    let output = ShadowOutput::MakePeaceEligibility(crate::make_peace_action_eligible(facts)?);
    Ok(ShadowEvaluation {
        metadata: metadata.clone(),
        binding_scope: BindingScope::CopiedFacts,
        input_digest: digest,
        output,
    })
}
/// Cooperative value-only fence; native hook coverage is never inferred from it.
pub struct ShadowSession {
    metadata: ShadowMetadata,
    active: bool,
}
impl ShadowSession {
    pub fn new(metadata: ShadowMetadata) -> Self {
        Self {
            metadata,
            active: true,
        }
    }
    pub fn evaluate(&mut self, input: &ShadowInput) -> Result<ShadowEvaluation, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let result = evaluate_shadow(&self.metadata, input);
        if result.is_err() {
            self.active = false;
        }
        result
    }
    /// Explicit supplied-fact action diagnostic; no world publication or native authority.
    pub fn evaluate_action(
        &mut self,
        facts: &crate::MakePeaceEligibility,
    ) -> Result<ShadowEvaluation, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let result = evaluate_action_shadow(&self.metadata, facts);
        if result.is_err() {
            self.active = false;
        }
        result
    }
    pub fn invalidate(&mut self) {
        self.active = false;
    }
    pub fn disable_provider(&mut self) {
        self.active = false;
    }
    /// Checks current supplied action facts; copied diagnostics never confer world authority.
    pub fn accept_action<'a>(
        &self,
        result: &'a ShadowEvaluation,
        current_metadata: &ShadowMetadata,
        current: &crate::MakePeaceEligibility,
    ) -> Result<&'a ShadowOutput, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let current_digest = crate::action_input_digest(current_metadata, current)?;
        if !matches!(&result.output, ShadowOutput::MakePeaceEligibility(_)) {
            return Err(Unavailable::Unsupported);
        }
        if self.metadata != result.metadata
            || self.metadata != *current_metadata
            || current_digest != result.input_digest
        {
            return Err(Unavailable::Stale);
        }
        if result.binding_scope != BindingScope::CopiedFacts {
            return Err(Unavailable::MissingFact);
        }
        Ok(&result.output)
    }
    /// Accepts a copied-facts diagnostic only after checking the actual current owner binding.
    pub fn accept<'a>(
        &self,
        result: &'a ShadowEvaluation,
        current_metadata: &ShadowMetadata,
        current: &ShadowInput,
    ) -> Result<&'a ShadowOutput, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let current_digest = input_digest(current_metadata, current)?;
        if self.metadata != result.metadata
            || self.metadata != *current_metadata
            || current_digest != result.input_digest
        {
            return Err(Unavailable::Stale);
        }
        if result.binding_scope != BindingScope::CopiedFacts {
            return Err(Unavailable::MissingFact);
        }
        Ok(&result.output)
    }
}
