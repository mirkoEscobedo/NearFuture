use super::{counters::Counts, PortableCompletionV1, ProducerNativeOriginV1,
    ProducerObservationOriginV1, ProducerScopeV1, RegisteredProducerCodeV1,
    RegisteredProducerKindV1, RegisteredProducerSeverityV1};
use nf_nex_shadow::{DiagnosticDroppedCountV1, DiagnosticStreamObservationV1, Difference, ShadowAuthority};

pub struct PortableEvaluationReceipt {
    completion: PortableCompletionV1,
    codes: Vec<RegisteredProducerCodeV1>,
}
impl PortableEvaluationReceipt {
    pub(super) fn new(differences: &[Difference]) -> Self {
        Self {
            completion: if differences.is_empty() {
                PortableCompletionV1::Matched
            } else {
                PortableCompletionV1::Mismatched
            },
            codes: differences.iter().map(|d| facet(*d).0).collect(),
        }
    }
    pub fn authority(&self) -> ShadowAuthority { ShadowAuthority::ShadowOnly }
    pub fn scope(&self) -> ProducerScopeV1 {
        ProducerScopeV1::PortableCopiedFactsSinceOwnerStart
    }
    pub fn completion(&self) -> PortableCompletionV1 { self.completion }
    pub fn codes(&self) -> &[RegisteredProducerCodeV1] { &self.codes }
}
safe_debug!(PortableEvaluationReceipt);

pub struct ProducerEvaluationCountsV1 {
    started: u64,
    completed: u64,
    unavailable: u64,
    matched: u64,
    mismatched: u64,
}
impl ProducerEvaluationCountsV1 {
    fn new(counts: &Counts) -> Self {
        Self { started: counts.started, completed: counts.completed,
            unavailable: counts.unavailable, matched: counts.matched,
            mismatched: counts.mismatched }
    }
    pub fn started(&self) -> u64 { self.started }
    pub fn completed(&self) -> u64 { self.completed }
    pub fn unavailable(&self) -> u64 { self.unavailable }
    pub fn matched(&self) -> u64 { self.matched }
    pub fn mismatched(&self) -> u64 { self.mismatched }
}
safe_debug!(ProducerEvaluationCountsV1);

pub struct RegisteredProducerRecordV1 {
    kind: RegisteredProducerKindV1,
    severity: RegisteredProducerSeverityV1,
    code: RegisteredProducerCodeV1,
    difference: Option<Difference>,
    count: u64,
}
impl RegisteredProducerRecordV1 {
    pub fn kind(&self) -> RegisteredProducerKindV1 { self.kind }
    pub fn severity(&self) -> RegisteredProducerSeverityV1 { self.severity }
    pub fn code(&self) -> RegisteredProducerCodeV1 { self.code }
    pub fn difference(&self) -> Option<Difference> { self.difference }
    pub fn count(&self) -> u64 { self.count }
}
safe_debug!(RegisteredProducerRecordV1);

pub struct ProducerObservationV1 {
    evaluations: ProducerEvaluationCountsV1,
    records: Vec<RegisteredProducerRecordV1>,
    l1: Option<DiagnosticStreamObservationV1>,
}
impl ProducerObservationV1 {
    pub(super) fn new(counts: &Counts, l1: Option<DiagnosticStreamObservationV1>) -> Self {
        let differences = [Difference::Kind, Difference::Lifecycle, Difference::Priority,
            Difference::Decision, Difference::Draws, Difference::Effects];
        let mut records = Vec::with_capacity(8);
        for (difference, count) in differences.into_iter().zip(counts.facets) {
            if count != 0 {
                let (code, difference) = facet(difference);
                records.push(RegisteredProducerRecordV1 {
                    kind: RegisteredProducerKindV1::NexCopiedFactsV1,
                    severity: RegisteredProducerSeverityV1::Warning,
                    code, difference: Some(difference), count,
                });
            }
        }
        for (code, severity, count) in [
            (RegisteredProducerCodeV1::ProducerEvaluationUnavailable,
                RegisteredProducerSeverityV1::Warning, counts.failures),
            (RegisteredProducerCodeV1::ProducerSessionRevoked,
                RegisteredProducerSeverityV1::Info, counts.revocations),
        ] {
            if count != 0 {
                records.push(RegisteredProducerRecordV1 {
                    kind: RegisteredProducerKindV1::ProducerLifecycleV1,
                    severity, code, difference: None, count,
                });
            }
        }
        Self { evaluations: ProducerEvaluationCountsV1::new(counts), records, l1 }
    }
    pub fn origin(&self) -> ProducerObservationOriginV1 {
        ProducerObservationOriginV1::ExistingOwnerGuardObservation
    }
    pub fn scope(&self) -> ProducerScopeV1 {
        ProducerScopeV1::PortableCopiedFactsSinceOwnerStart
    }
    pub fn authority(&self) -> ShadowAuthority { ShadowAuthority::ShadowOnly }
    pub fn native_origin(&self) -> ProducerNativeOriginV1 {
        ProducerNativeOriginV1::Unqualified
    }
    pub fn evaluations(&self) -> &ProducerEvaluationCountsV1 { &self.evaluations }
    pub fn records(&self) -> &[RegisteredProducerRecordV1] { &self.records }
    pub fn l1(&self) -> Option<&DiagnosticStreamObservationV1> { self.l1.as_ref() }
    pub fn dropped(&self) -> Option<DiagnosticDroppedCountV1> {
        self.l1.as_ref().map(DiagnosticStreamObservationV1::dropped)
    }
    pub fn coalesced_records(&self) -> Option<u64> { None }
    pub fn historical_records(&self) -> Option<u64> { None }
}
safe_debug!(ProducerObservationV1);

fn facet(difference: Difference) -> (RegisteredProducerCodeV1, Difference) {
    let code = match difference {
        Difference::Kind => RegisteredProducerCodeV1::NexKindMismatch,
        Difference::Lifecycle => RegisteredProducerCodeV1::NexLifecycleMismatch,
        Difference::Priority => RegisteredProducerCodeV1::NexPriorityMismatch,
        Difference::Decision => RegisteredProducerCodeV1::NexDecisionMismatch,
        Difference::Draws => RegisteredProducerCodeV1::NexDrawsMismatch,
        Difference::Effects => RegisteredProducerCodeV1::NexEffectsMismatch,
    };
    (code, difference)
}
