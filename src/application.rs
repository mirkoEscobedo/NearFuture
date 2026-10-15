//! Actual portable Rust work over unverified copied facts, never native authority.
macro_rules! safe_debug {
    ($name:ident) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(data_only,evidence_is_not_authority)"))
            }
        }
    };
}
mod admission;
mod counters;
mod enums;
mod observation;
mod producer;
pub use enums::*;
pub use observation::{PortableEvaluationReceipt, ProducerEvaluationCountsV1,
    ProducerObservationV1, RegisteredProducerRecordV1};
use nf_nex_shadow::{MakePeaceEligibility, ShadowInput, ShadowMetadata, ShadowOutput};
use producer::{Producer, Work};
use std::{marker::PhantomData, rc::Rc, thread::{self, ThreadId}, time::{Duration, Instant}};

/// Untrusted request data, not a source, installed package or owner witness.
pub struct ForegroundRequest {
    pub lifetime_millis: u32,
    pub package_scope: PackageObservationScopeV1,
}
pub enum PortableAdmissionInputV1<'a> {
    Copied(&'a ShadowInput),
    Action(&'a MakePeaceEligibility),
}
pub struct ProducerObservationSelection {
    pub required_scope: ProducerScopeV1,
    pub include_l1: bool,
    pub include_metrics: bool,
}
enum Lifecycle { Running, Revoking, Stopped }

/// Owns only portable sessions and bounded RAM. It launches no thread or process.
pub struct ForegroundApplication {
    started: Instant,
    lifetime: Duration,
    thread: ThreadId,
    _instance: [[u8; 16]; 2],
    generation: u64,
    lifecycle: Lifecycle,
    producer: Option<Producer>,
    _owner_thread_only: PhantomData<Rc<()>>,
}
safe_debug!(ForegroundApplication);

pub fn start_foreground(request: ForegroundRequest) -> Result<ForegroundApplication, HostError> {
    if !(100..=900_000).contains(&request.lifetime_millis) {
        return Err(HostError::InvalidRequest);
    }
    match request.package_scope { PackageObservationScopeV1::WorkspacePublicSubset => {} }
    let started = Instant::now();
    let owner_thread = thread::current().id();
    let lifetime = Duration::from_millis(u64::from(request.lifetime_millis));
    let first = nf_identity::keys::random_id().map_err(|_| HostError::Io)?;
    if started.elapsed() >= lifetime { return Err(HostError::Expired); }
    let second = nf_identity::keys::random_id().map_err(|_| HostError::Io)?;
    if started.elapsed() >= lifetime { return Err(HostError::Expired); }
    Ok(ForegroundApplication {
        started, lifetime, thread: owner_thread, _instance: [first, second], generation: 0,
        lifecycle: Lifecycle::Running, producer: None, _owner_thread_only: PhantomData,
    })
}
impl ForegroundApplication {
    fn shutdown(&mut self) -> Result<(), HostError> {
        self.lifecycle = Lifecycle::Revoking;
        let result = self.producer.as_mut().map_or(Ok(()), Producer::revoke);
        self.producer = None;
        self.lifecycle = Lifecycle::Stopped;
        result
    }
    fn gate(&mut self) -> Result<(), HostError> {
        if thread::current().id() != self.thread {
            let _ = self.shutdown();
            return Err(HostError::WrongThread);
        }
        if !matches!(self.lifecycle, Lifecycle::Running) { return Err(HostError::Inactive); }
        if self.started.elapsed() >= self.lifetime {
            let _ = self.shutdown();
            return Err(HostError::Expired);
        }
        Ok(())
    }
    fn advance(&mut self) -> Result<(), HostError> {
        match self.generation.checked_add(1) {
            Some(next) => { self.generation = next; Ok(()) }
            None => { let _ = self.shutdown(); Err(HostError::Bounds) }
        }
    }
    pub fn register_portable_session(&mut self, metadata: &ShadowMetadata,
        first_input: PortableAdmissionInputV1<'_>) -> Result<(), HostError>
    {
        self.gate()?;
        if self.producer.is_some() { return Err(HostError::InvalidRequest); }
        let admission = admission::canonical(metadata, first_input);
        self.gate()?;
        admission?;
        let constructed = Producer::new(metadata);
        self.gate()?;
        let producer = constructed?;
        self.advance()?;
        self.producer = Some(producer);
        Ok(())
    }
    pub fn evaluate_copied(&mut self, input: &ShadowInput, reference: &ShadowOutput)
        -> Result<PortableEvaluationReceipt, HostError>
    {
        self.gate()?;
        admission::copied(input)?;
        self.evaluate(Work::Copied(input), reference)
    }
    pub fn evaluate_action_copied(&mut self, input: &MakePeaceEligibility, reference: &ShadowOutput)
        -> Result<PortableEvaluationReceipt, HostError>
    {
        self.evaluate(Work::Action(input), reference)
    }
    fn evaluate(&mut self, work: Work<'_>, reference: &ShadowOutput)
        -> Result<PortableEvaluationReceipt, HostError>
    {
        self.gate()?;
        let producer = self.producer.as_ref().ok_or(HostError::UnsupportedSource)?;
        producer.require_active()?;
        if let Err(error) = producer.reserve_attempt() {
            let _ = self.shutdown();
            return Err(error);
        }
        self.advance()?;
        let evaluated = self.producer.as_mut().ok_or(HostError::Inactive)?.evaluate(work);
        self.gate()?;
        let evaluation = match evaluated {
            Ok(value) => value,
            Err(_) => return self.evaluation_failed(),
        };
        let accepted = self.producer.as_ref().ok_or(HostError::Inactive)?
            .accept(work, &evaluation).map(|_| ());
        self.gate()?;
        if accepted.is_err() { return self.evaluation_failed(); }
        let completed = self.producer.as_mut().ok_or(HostError::Inactive)?
            .complete(&evaluation, reference);
        self.gate()?;
        match completed {
            Ok(receipt) => Ok(receipt),
            Err(error) => { let _ = self.shutdown(); Err(error) }
        }
    }
    fn evaluation_failed(&mut self) -> Result<PortableEvaluationReceipt, HostError> {
        let failed = self.producer.as_mut().ok_or(HostError::Inactive)?.failed();
        if let Err(error) = failed {
            let _ = self.shutdown();
            return Err(error);
        }
        self.gate()?;
        Err(HostError::EvaluationUnavailable)
    }
    pub fn observe_producer(&mut self, selection: ProducerObservationSelection)
        -> Result<ProducerObservationV1, HostError>
    {
        self.gate()?;
        if selection.required_scope == ProducerScopeV1::NativeAdmittedProducerSinceOwnerStart {
            return Err(HostError::NativeAdmissionUnavailable);
        }
        if selection.include_metrics { return Err(HostError::UnsupportedSource); }
        let observation = self.producer.as_ref().ok_or(HostError::UnsupportedSource)?
            .observe(selection.include_l1);
        self.gate()?;
        Ok(observation)
    }
    pub fn stop(mut self) -> Result<HostExit, HostError> {
        match self.gate() {
            Ok(()) => { self.shutdown()?; Ok(HostExit::Stopped) }
            Err(HostError::Expired) => Ok(HostExit::Expired),
            Err(error) => Err(error),
        }
    }
}
impl Drop for ForegroundApplication {
    fn drop(&mut self) {
        // Private release only; no checked OS close or descendant-zero claim.
        let _ = self.shutdown();
    }
}
