use crate::{ShadowEvaluation, ShadowInput, ShadowMetadata, ShadowOutput, Unavailable};
use alloc::{collections::VecDeque, vec::Vec};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Difference {
    Kind,
    Lifecycle,
    Priority,
    Decision,
    Draws,
    Effects,
    FirstConcernKind,
    FirstConcernLifecycle,
    FirstConcernPriority,
    FirstEligibilityKind,
    FirstEligibilityDecision,
    FirstActionPriority,
    FirstProposalKind,
    FirstProposalDecision,
    FirstProposalDraws,
    FirstProposalEffects,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub source_digest: [u8; 32],
    pub config_digest: [u8; 32],
    pub input_digest: [u8; 32],
    pub corpus_digest: [u8; 32],
    pub implementation_digest: [u8; 32],
    pub difference: Difference,
}
impl Diagnostic {
    pub fn from_evaluation(result: &ShadowEvaluation, difference: Difference) -> Self {
        let m = result.metadata();
        Self {
            source_digest: m.provenance.source_digest,
            config_digest: m.provenance.merged_config_digest,
            input_digest: result.input_digest(),
            corpus_digest: m.corpus_digest,
            implementation_digest: m.implementation_digest,
            difference,
        }
    }
}
/// Bounded optional diagnostics; equal records coalesce, overflow is explicitly counted.
pub struct DiagnosticStream {
    queue: VecDeque<Diagnostic>,
    capacity: usize,
    byte_limit: usize,
    dropped: u64,
}
impl DiagnosticStream {
    pub fn new(capacity: usize, byte_limit: usize) -> Result<Self, Unavailable> {
        if capacity == 0 || capacity > 64 || !(161..=65536).contains(&byte_limit) {
            return Err(Unavailable::Limit);
        }
        Ok(Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            byte_limit,
            dropped: 0,
        })
    }
    pub fn push(&mut self, value: Diagnostic) {
        if self.queue.contains(&value) {
            return;
        }
        if self.queue.len() >= self.capacity || (self.queue.len() + 1) * 161 > self.byte_limit {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        self.queue.push_back(value);
    }
    pub fn pop(&mut self) -> Option<Diagnostic> {
        self.queue.pop_front()
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}
pub fn compare_outputs(expected: &ShadowOutput, actual: &ShadowOutput) -> Vec<Difference> {
    let mut result = Vec::with_capacity(3);
    match (expected, actual) {
        (ShadowOutput::War(a), ShadowOutput::War(b)) => {
            if (a.generated, a.ended, a.abort_current_action, a.valid)
                != (b.generated, b.ended, b.abort_current_action, b.valid)
            {
                result.push(Difference::Lifecycle);
            }
            if a.existing_priority != b.existing_priority || a.writes != b.writes {
                result.push(Difference::Priority);
            }
        }
        (ShadowOutput::SelectedPeace(a), ShadowOutput::SelectedPeace(b)) => {
            if a.decision != b.decision {
                result.push(Difference::Decision);
            }
            if a.consumed_draws != b.consumed_draws {
                result.push(Difference::Draws);
            }
            if a.effects != b.effects {
                result.push(Difference::Effects);
            }
        }
        (ShadowOutput::MakePeaceEligibility(a), ShadowOutput::MakePeaceEligibility(b)) => {
            if a != b {
                result.push(Difference::Decision);
            }
        }
        _ => result.push(Difference::Kind),
    }
    result
}
/// Explicit caller-requested bytes only. No automatic file write or captured/private export exists.
/// Observation is supplied evidence, not an access-control classification; caller must review synthetic input.
pub fn synthetic_reproduction(
    m: &ShadowMetadata,
    input: &ShadowInput,
) -> Result<Vec<u8>, Unavailable> {
    if m.provenance.observation != nf_nex_boundary::Observation::Synthetic {
        return Err(Unavailable::Unsupported);
    }
    crate::encode_input(m, input)
}

/// Independent reference observations; wrong facet kinds remain explicit differences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstProposalObservation {
    pub concern: ShadowOutput,
    pub eligibility: ShadowOutput,
    pub additional_priority: nf_nex_boundary::Modifier,
    pub selected: ShadowOutput,
}
/// At most seven differences; facet identity survives DiagnosticStream coalescing.
pub fn compare_first_proposal_outputs(
    expected: &FirstProposalObservation,
    actual: &crate::FirstProposalOutput,
) -> Vec<Difference> {
    let mut differences = Vec::with_capacity(7);
    for difference in compare_outputs(
        &expected.concern,
        &ShadowOutput::War(actual.concern.clone()),
    ) {
        differences.push(match difference {
            Difference::Lifecycle => Difference::FirstConcernLifecycle,
            Difference::Priority => Difference::FirstConcernPriority,
            _ => Difference::FirstConcernKind,
        });
    }
    for difference in compare_outputs(
        &expected.eligibility,
        &ShadowOutput::MakePeaceEligibility(actual.eligible),
    ) {
        differences.push(match difference {
            Difference::Decision => Difference::FirstEligibilityDecision,
            _ => Difference::FirstEligibilityKind,
        });
    }
    if expected.additional_priority != actual.additional_priority {
        differences.push(Difference::FirstActionPriority);
    }
    for difference in compare_outputs(
        &expected.selected,
        &ShadowOutput::SelectedPeace(actual.selected.clone()),
    ) {
        differences.push(match difference {
            Difference::Decision => Difference::FirstProposalDecision,
            Difference::Draws => Difference::FirstProposalDraws,
            Difference::Effects => Difference::FirstProposalEffects,
            _ => Difference::FirstProposalKind,
        });
    }
    differences
}
impl Diagnostic {
    pub fn from_first_proposal(
        result: &crate::FirstProposalEvaluation,
        difference: Difference,
    ) -> Self {
        let metadata = result.metadata();
        Self {
            source_digest: metadata.provenance.source_digest,
            config_digest: metadata.provenance.merged_config_digest,
            input_digest: result.input_digest(),
            corpus_digest: metadata.corpus_digest,
            implementation_digest: metadata.implementation_digest,
            difference,
        }
    }
}
/// Value-only public synthetic bytes. Persistence is an explicit confined host responsibility.
pub fn synthetic_first_proposal_reproduction(
    metadata: &ShadowMetadata,
    input: &crate::FirstProposalInput,
) -> Result<Vec<u8>, Unavailable> {
    if metadata.provenance.observation != nf_nex_boundary::Observation::Synthetic {
        return Err(Unavailable::Unsupported);
    }
    crate::encode_first_proposal_input(metadata, input)
}
