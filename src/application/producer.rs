use super::{
    HostError, ProducerObservationV1,
    counters::Counts,
    metrics::{self, Window},
    observation::PortableEvaluationReceipt,
};
use nf_nex_shadow::{
    Diagnostic, DiagnosticStream, MakePeaceEligibility, ShadowEvaluation, ShadowInput,
    ShadowMetadata, ShadowOutput, ShadowSession, compare_outputs,
};
use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) enum Work<'a> {
    Copied(&'a ShadowInput),
    Action(&'a MakePeaceEligibility),
}

/// Only the actual evaluation edge can create this private timing/result pair.
pub(super) struct Evaluation {
    value: ShadowEvaluation,
    started: Instant,
}

pub(super) struct Producer {
    // Current binding and the maintained session each own one bounded metadata copy.
    metadata: ShadowMetadata,
    session: ShadowSession,
    stream: DiagnosticStream,
    counts: Counts,
    metrics: Window,
    active: bool,
}
impl Producer {
    pub fn new(metadata: &ShadowMetadata) -> Result<Self, HostError> {
        let stream = DiagnosticStream::new(64, 10_304).map_err(|_| HostError::Bounds)?;
        Ok(Self {
            metadata: metadata.clone(),
            session: ShadowSession::new(metadata.clone()),
            stream,
            counts: Counts::default(),
            metrics: Window::new(),
            active: true,
        })
    }
    pub fn require_active(&self) -> Result<(), HostError> {
        if self.active {
            Ok(())
        } else {
            Err(HostError::EvaluationUnavailable)
        }
    }
    pub fn reserve_attempt(&self) -> Result<(), HostError> {
        self.counts.reserve_attempt()?;
        self.metrics.preflight(self.counts.completed)
    }
    pub fn evaluate(&mut self, work: Work<'_>) -> Result<Evaluation, HostError> {
        // No caller clock or failed-attempt sample. Both real routes start at their edge.
        let (started, result) = match work {
            Work::Copied(input) => {
                let started = Instant::now();
                let result = self.session.evaluate(input);
                (started, result)
            }
            Work::Action(input) => {
                let started = Instant::now();
                let result = self.session.evaluate_action(input);
                (started, result)
            }
        };
        let value = result.map_err(|_| HostError::EvaluationUnavailable)?;
        Ok(Evaluation { value, started })
    }
    pub fn accept<'a>(
        &self,
        work: Work<'_>,
        result: &'a Evaluation,
    ) -> Result<&'a ShadowOutput, HostError> {
        let accepted = match work {
            Work::Copied(input) => self.session.accept(&result.value, &self.metadata, input),
            Work::Action(input) => self
                .session
                .accept_action(&result.value, &self.metadata, input),
        };
        accepted.map_err(|_| HostError::EvaluationUnavailable)
    }
    pub fn complete(
        &mut self,
        evaluation: &Evaluation,
        reference: &ShadowOutput,
    ) -> Result<PortableEvaluationReceipt, HostError> {
        // Unverified reference data; this is the one actual maintained comparison.
        // Actual outputs are bounded: unequal lengths refuse equality before payload bytes.
        let differences = compare_outputs(reference, evaluation.value.output());
        let finished = Instant::now();
        let duration = finished
            .checked_duration_since(evaluation.started)
            .ok_or(HostError::SourceChanged)?;
        let nanos = u64::try_from(duration.as_nanos()).map_err(|_| HostError::Bounds)?;
        // The endpoint above excludes counter updates, queue pushes, append and receipt creation.
        let next = self.counts.completed(&differences)?;
        next.validate()?;
        self.metrics
            .append(nanos, self.counts.completed, next.completed)?;
        for difference in &differences {
            self.stream
                .push(Diagnostic::from_evaluation(&evaluation.value, *difference));
        }
        self.counts = next;
        Ok(PortableEvaluationReceipt::new(&differences))
    }
    pub fn failed(&mut self) -> Result<(), HostError> {
        self.session.disable_provider();
        self.active = false;
        self.counts = self.counts.unavailable()?;
        Ok(())
    }
    pub fn observe(
        &self,
        include_l1: bool,
        include_metrics: bool,
    ) -> Result<ProducerObservationV1, HostError> {
        self.counts.validate()?;
        self.metrics.validate(self.counts.completed)?;
        let metrics = if include_metrics {
            Some(metrics::observe(&self.metrics, self.counts.completed)?)
        } else {
            None
        };
        let queue = include_l1.then(|| self.stream.redacted_observation());
        Ok(ProducerObservationV1::new(&self.counts, queue, metrics))
    }
    pub fn revoke(&mut self) -> Result<(), HostError> {
        self.session.invalidate();
        if self.active {
            self.active = false;
            self.counts = self.counts.revoked()?;
        }
        Ok(())
    }
}
impl Drop for Producer {
    fn drop(&mut self) {
        // Release-only fallback; no successful stop/resource-zero receipt.
        self.session.invalidate();
        self.active = false;
    }
}
