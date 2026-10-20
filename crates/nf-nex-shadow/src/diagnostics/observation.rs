use super::{DiagnosticStream, Difference};
use alloc::vec::Vec;

/// Public stream inputs do not establish an authenticated producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticObservationOriginV1 {
    ExternallySuppliedUnverified,
}

/// Observation covers the records currently retained by this stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticObservationScopeV1 {
    QueuedStreamContentsOnly,
}

/// A saturated dropped counter is only a lower bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticDroppedCountV1 {
    Exact(u64),
    AtLeast(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticObservationUnavailableV1 {
    ProducerBinding,
    ClockAlignment,
    Metrics,
    HistoricalRecords,
    CoalescedRecords,
}

const UNAVAILABLE: [DiagnosticObservationUnavailableV1; 5] = [
    DiagnosticObservationUnavailableV1::ProducerBinding,
    DiagnosticObservationUnavailableV1::ClockAlignment,
    DiagnosticObservationUnavailableV1::Metrics,
    DiagnosticObservationUnavailableV1::HistoricalRecords,
    DiagnosticObservationUnavailableV1::CoalescedRecords,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticDifferenceCountV1 {
    difference: Difference,
    count: u64,
}

impl DiagnosticDifferenceCountV1 {
    pub fn difference(&self) -> Difference {
        self.difference
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Owned redacted counts, with no record identity or source capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticStreamObservationV1 {
    queued_records: u64,
    difference_counts: Vec<DiagnosticDifferenceCountV1>,
    dropped: DiagnosticDroppedCountV1,
}

impl DiagnosticStreamObservationV1 {
    pub fn origin(&self) -> DiagnosticObservationOriginV1 {
        DiagnosticObservationOriginV1::ExternallySuppliedUnverified
    }

    pub fn scope(&self) -> DiagnosticObservationScopeV1 {
        DiagnosticObservationScopeV1::QueuedStreamContentsOnly
    }

    /// Every currently queued record was counted; this does not cover history.
    pub fn complete_within_scope(&self) -> bool {
        true
    }

    pub fn queued_records(&self) -> u64 {
        self.queued_records
    }

    pub fn difference_counts(&self) -> &[DiagnosticDifferenceCountV1] {
        &self.difference_counts
    }

    pub fn dropped(&self) -> DiagnosticDroppedCountV1 {
        self.dropped
    }

    pub fn coalesced_records(&self) -> Option<u64> {
        None
    }

    pub fn historical_records(&self) -> Option<u64> {
        None
    }

    pub fn unavailable(&self) -> &[DiagnosticObservationUnavailableV1] {
        &UNAVAILABLE
    }
}

impl DiagnosticStream {
    /// Count the actual retained queue without draining it or exposing identities.
    pub fn redacted_observation(&self) -> DiagnosticStreamObservationV1 {
        let mut counts = [0_u64; 16];
        for diagnostic in &self.queue {
            let index = match diagnostic.difference {
                Difference::Kind => 0,
                Difference::Lifecycle => 1,
                Difference::Priority => 2,
                Difference::Decision => 3,
                Difference::Draws => 4,
                Difference::Effects => 5,
                Difference::FirstConcernKind => 6,
                Difference::FirstConcernLifecycle => 7,
                Difference::FirstConcernPriority => 8,
                Difference::FirstEligibilityKind => 9,
                Difference::FirstEligibilityDecision => 10,
                Difference::FirstActionPriority => 11,
                Difference::FirstProposalKind => 12,
                Difference::FirstProposalDecision => 13,
                Difference::FirstProposalDraws => 14,
                Difference::FirstProposalEffects => 15,
            };
            counts[index] += 1;
        }

        let differences = [
            Difference::Kind,
            Difference::Lifecycle,
            Difference::Priority,
            Difference::Decision,
            Difference::Draws,
            Difference::Effects,
            Difference::FirstConcernKind,
            Difference::FirstConcernLifecycle,
            Difference::FirstConcernPriority,
            Difference::FirstEligibilityKind,
            Difference::FirstEligibilityDecision,
            Difference::FirstActionPriority,
            Difference::FirstProposalKind,
            Difference::FirstProposalDecision,
            Difference::FirstProposalDraws,
            Difference::FirstProposalEffects,
        ];
        let mut difference_counts = Vec::with_capacity(16);
        for (difference, count) in differences.into_iter().zip(counts) {
            if count != 0 {
                difference_counts.push(DiagnosticDifferenceCountV1 { difference, count });
            }
        }

        DiagnosticStreamObservationV1 {
            queued_records: self.queue.len() as u64,
            difference_counts,
            dropped: if self.dropped == u64::MAX {
                DiagnosticDroppedCountV1::AtLeast(u64::MAX)
            } else {
                DiagnosticDroppedCountV1::Exact(self.dropped)
            },
        }
    }
}
