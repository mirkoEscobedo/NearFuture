use super::{
    HostError, ProducerObservationV1, counters::Counts, observation::PortableEvaluationReceipt,
};
use nf_nex_shadow::{
    Diagnostic, DiagnosticStream, MakePeaceEligibility, ShadowEvaluation, ShadowInput,
    ShadowMetadata, ShadowOutput, ShadowSession, compare_outputs,
};

#[derive(Clone, Copy)]
pub(super) enum Work<'a> {
    Copied(&'a ShadowInput),
    Action(&'a MakePeaceEligibility),
}
pub(super) struct Producer {
    // Two bounded metadata copies are needed by the maintained acceptance API:
    // the immutable current binding here and the session's own private binding.
    metadata: ShadowMetadata,
    session: ShadowSession,
    stream: DiagnosticStream,
    counts: Counts,
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
        self.counts.reserve_attempt()
    }
    pub fn evaluate(&mut self, work: Work<'_>) -> Result<ShadowEvaluation, HostError> {
        let result = match work {
            Work::Copied(input) => self.session.evaluate(input),
            Work::Action(input) => self.session.evaluate_action(input),
        };
        result.map_err(|_| HostError::EvaluationUnavailable)
    }
    pub fn accept<'a>(
        &self,
        work: Work<'_>,
        result: &'a ShadowEvaluation,
    ) -> Result<&'a ShadowOutput, HostError> {
        let accepted = match work {
            Work::Copied(input) => self.session.accept(result, &self.metadata, input),
            Work::Action(input) => self.session.accept_action(result, &self.metadata, input),
        };
        accepted.map_err(|_| HostError::EvaluationUnavailable)
    }
    pub fn complete(
        &mut self,
        evaluation: &ShadowEvaluation,
        reference: &ShadowOutput,
    ) -> Result<PortableEvaluationReceipt, HostError> {
        // The caller supplies unverified reference data; comparison is actual Rust work.
        // Equality against the bounded actual output cannot walk a larger reference
        // vector/string: mismatched lengths refuse equality before element bytes.
        let differences = compare_outputs(reference, evaluation.output());
        let next = self.counts.completed(&differences)?;
        let receipt = PortableEvaluationReceipt::new(&differences);
        for difference in differences {
            self.stream
                .push(Diagnostic::from_evaluation(evaluation, difference));
        }
        self.counts = next;
        Ok(receipt)
    }
    pub fn failed(&mut self) -> Result<(), HostError> {
        self.session.disable_provider();
        self.active = false;
        self.counts = self.counts.unavailable()?;
        Ok(())
    }
    pub fn observe(&self, include_l1: bool) -> ProducerObservationV1 {
        let queue = include_l1.then(|| self.stream.redacted_observation());
        ProducerObservationV1::new(&self.counts, queue)
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
