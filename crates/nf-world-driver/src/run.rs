use crate::{Driver, DriverError, Pacer};
use nf_store::miniature::MiniatureStoreError;
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStage {
    PendingPrepared,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStop {
    Deadline,
    Inactive,
    Pending,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunReport {
    pub committed_ticks: u64,
    pub stop: RunStop,
}
impl Driver {
    /// Already-claimed foreground loop. Pacing grants no authority and does not create a claim.
    pub fn run_for(
        &mut self,
        period_ms: u64,
        duration_seconds: u64,
        active: bool,
    ) -> Result<RunReport, DriverError> {
        self.run_for_with_observer(period_ms, duration_seconds, active, &mut |_| {})
    }
    /// Explicit owner operation: the same private window covers the fresh claim and all new mutations.
    pub fn run_as_owner(
        &mut self,
        period_ms: u64,
        duration_seconds: u64,
        active: bool,
    ) -> Result<RunReport, DriverError> {
        self.run_bounded(period_ms, duration_seconds, active, true, &mut |_| {})
    }
    /// Read-only progress callbacks consume the window and cannot extend it or authorize a mutation.
    pub fn run_for_with_observer(
        &mut self,
        period_ms: u64,
        duration_seconds: u64,
        active: bool,
        observer: &mut impl FnMut(RunStage),
    ) -> Result<RunReport, DriverError> {
        self.run_bounded(period_ms, duration_seconds, active, false, observer)
    }
    fn run_bounded(
        &mut self,
        period_ms: u64,
        duration_seconds: u64,
        active: bool,
        claim: bool,
        observer: &mut impl FnMut(RunStage),
    ) -> Result<RunReport, DriverError> {
        self.run_deadline = None;
        if !(100..=60000).contains(&period_ms) || !(1..=60).contains(&duration_seconds) {
            return Err(DriverError::InvalidCommand);
        }
        let origin = Instant::now();
        let duration = Duration::from_secs(duration_seconds);
        self.run_deadline = Some(origin.checked_add(duration).ok_or(DriverError::Limit)?);
        let result = self.run_window(period_ms, duration, origin, active, claim, observer);
        self.run_deadline = None;
        result
    }
    fn run_window(
        &mut self,
        period_ms: u64,
        duration: Duration,
        origin: Instant,
        active: bool,
        claim: bool,
        observer: &mut impl FnMut(RunStage),
    ) -> Result<RunReport, DriverError> {
        let mut report = RunReport {
            committed_ticks: 0,
            stop: RunStop::Deadline,
        };
        if !active {
            report.stop = RunStop::Inactive;
            return Ok(report);
        }
        if self.store.pending().is_some() {
            report.stop = RunStop::Pending;
            return Ok(report);
        }
        if claim && let Err(error) = self.claim_authority() {
            if self.is_window_failure(error) {
                return Ok(report);
            }
            return Err(error);
        }
        let now = u64::try_from(origin.elapsed().as_millis()).map_err(|_| DriverError::Limit)?;
        let mut pacer = Pacer::new(period_ms, now).map_err(|_| DriverError::Limit)?;
        loop {
            let elapsed = origin.elapsed();
            if elapsed >= duration {
                return Ok(report);
            }
            let now = u64::try_from(elapsed.as_millis()).map_err(|_| DriverError::Limit)?;
            if pacer
                .observe(now)
                .map_err(|_| DriverError::Limit)?
                .is_some()
            {
                match self.advance_empty_observed(true, observer) {
                    Ok(_) => {
                        report.committed_ticks = report
                            .committed_ticks
                            .checked_add(1)
                            .ok_or(DriverError::Limit)?
                    }
                    Err(error) if self.is_window_failure(error) => return Ok(report),
                    Err(error) => return Err(error),
                }
                let actual =
                    u64::try_from(origin.elapsed().as_millis()).map_err(|_| DriverError::Limit)?;
                pacer.rearm(actual).map_err(|_| DriverError::Limit)?;
            } else {
                std::thread::sleep(
                    Duration::from_millis(10).min(duration.saturating_sub(origin.elapsed())),
                );
            }
        }
    }
    fn is_window_failure(&self, error: DriverError) -> bool {
        self.budget_expired()
            && matches!(
                error,
                DriverError::WindowExpired | DriverError::Store(MiniatureStoreError::Expired)
            )
    }
}
