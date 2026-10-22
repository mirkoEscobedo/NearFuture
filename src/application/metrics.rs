//! Closed data from the actual portable producer's measured completed-work window.
use super::{HostError, ProducerObservationOriginV1};

mod window;
pub(super) use window::Window;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredProducerMetricV1 {
    RustShadowEvaluationDuration,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerMetricUnitV1 {
    Nanoseconds,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerMetricScopeV1 {
    RegisteredOwnerLast64CompletedEvaluationSamples,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerMetricClockV1 {
    LocalOwnerMonotonicElapsedUnaligned,
}

/// Owned scalar projection only; construction and raw samples stay private.
pub struct ProducerMetricObservationV1 {
    completed: u64,
    retained: u64,
    evicted: u64,
    quantiles: [Option<u64>; 4],
}
impl ProducerMetricObservationV1 {
    pub fn metric(&self) -> RegisteredProducerMetricV1 {
        RegisteredProducerMetricV1::RustShadowEvaluationDuration
    }
    pub fn unit(&self) -> ProducerMetricUnitV1 {
        ProducerMetricUnitV1::Nanoseconds
    }
    pub fn completed_samples(&self) -> u64 {
        self.completed
    }
    pub fn sample_count(&self) -> u64 {
        self.retained
    }
    pub fn evicted_samples(&self) -> u64 {
        self.evicted
    }
    pub fn p50(&self) -> Option<u64> {
        self.quantiles[0]
    }
    pub fn p95(&self) -> Option<u64> {
        self.quantiles[1]
    }
    pub fn p99(&self) -> Option<u64> {
        self.quantiles[2]
    }
    pub fn p999(&self) -> Option<u64> {
        self.quantiles[3]
    }
}
safe_debug!(ProducerMetricObservationV1);

/// Evidence of one registered metric, never a native or current-owner capability.
pub struct ProducerMetricsObservationV1 {
    rows: [ProducerMetricObservationV1; 1],
}
impl ProducerMetricsObservationV1 {
    pub fn origin(&self) -> ProducerObservationOriginV1 {
        ProducerObservationOriginV1::ExistingOwnerGuardObservation
    }
    pub fn scope(&self) -> ProducerMetricScopeV1 {
        ProducerMetricScopeV1::RegisteredOwnerLast64CompletedEvaluationSamples
    }
    pub fn clock(&self) -> ProducerMetricClockV1 {
        ProducerMetricClockV1::LocalOwnerMonotonicElapsedUnaligned
    }
    pub fn window_capacity(&self) -> u64 {
        64
    }
    pub fn complete_within_scope(&self) -> bool {
        true
    }
    pub fn metrics(&self) -> &[ProducerMetricObservationV1] {
        &self.rows
    }
}
safe_debug!(ProducerMetricsObservationV1);

pub(super) fn observe(
    window: &Window,
    expected_completed: u64,
) -> Result<ProducerMetricsObservationV1, HostError> {
    let summary = window.summary(expected_completed)?;
    Ok(ProducerMetricsObservationV1 {
        rows: [ProducerMetricObservationV1 {
            completed: summary.completed,
            retained: summary.retained,
            evicted: summary.evicted,
            quantiles: summary.quantiles,
        }],
    })
}
