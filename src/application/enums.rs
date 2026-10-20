//! Closed public data labels; naming one never creates an owner or observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostError {
    InvalidRequest,
    WrongThread,
    Bounds,
    Expired,
    Inactive,
    UnsupportedSource,
    NativeAdmissionUnavailable,
    Authorization,
    EvaluationUnavailable,
    SourceChanged,
    Io,
}
impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for HostError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostExit {
    Stopped,
    Expired,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageObservationScopeV1 {
    WorkspacePublicSubset,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortableCompletionV1 {
    Matched,
    Mismatched,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerObservationOriginV1 {
    ExistingOwnerGuardObservation,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerScopeV1 {
    PortableCopiedFactsSinceOwnerStart,
    NativeAdmittedProducerSinceOwnerStart,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerNativeOriginV1 {
    Unqualified,
    QualifiedNativeObservation,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredProducerKindV1 {
    NexCopiedFactsV1,
    NexDeclaredWorldV1,
    NexFirstProposalV1,
    ProducerLifecycleV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredProducerSeverityV1 {
    Info,
    Warning,
    Error,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredProducerCodeV1 {
    NexKindMismatch,
    NexLifecycleMismatch,
    NexPriorityMismatch,
    NexDecisionMismatch,
    NexDrawsMismatch,
    NexEffectsMismatch,
    NexFirstConcernKindMismatch,
    NexFirstConcernLifecycleMismatch,
    NexFirstConcernPriorityMismatch,
    NexFirstEligibilityKindMismatch,
    NexFirstEligibilityDecisionMismatch,
    NexFirstActionPriorityMismatch,
    NexFirstProposalKindMismatch,
    NexFirstProposalDecisionMismatch,
    NexFirstProposalDrawsMismatch,
    NexFirstProposalEffectsMismatch,
    ProducerEvaluationUnavailable,
    ProducerSessionRevoked,
}
