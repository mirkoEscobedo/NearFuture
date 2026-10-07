//! Checked monotonic scheduling data, with at most one outstanding due hint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacingError {
    InvalidPeriod,
    BackwardsObservation,
    Overflow,
    Poisoned,
}

/// An immutable scheduling observation. This is not an activity or commit certificate.
#[derive(Debug, Eq, PartialEq)]
pub struct DueHint {
    observed_ms: u64,
    deadline_ms: u64,
}
impl DueHint {
    pub fn observed_ms(&self) -> u64 {
        self.observed_ms
    }
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }
}

enum State {
    Armed { deadline_ms: u64 },
    Outstanding,
    Paused,
    Poisoned,
}

/// Caller observations must share one monotonic origin. The policy reads no clock.
/// Backwards observations or deadline overflow poison the instance permanently.
pub struct Pacer {
    period_ms: u64,
    last_observation_ms: u64,
    state: State,
}
impl Pacer {
    pub fn new(period_ms: u64, now_ms: u64) -> Result<Self, PacingError> {
        if !(100..=60_000).contains(&period_ms) {
            return Err(PacingError::InvalidPeriod);
        }
        let deadline_ms = now_ms.checked_add(period_ms).ok_or(PacingError::Overflow)?;
        Ok(Self {
            period_ms,
            last_observation_ms: now_ms,
            state: State::Armed { deadline_ms },
        })
    }

    /// A large elapsed gap emits one hint and never advances any strategic clock.
    /// Further observations cannot emit another until an explicit rearm.
    pub fn observe(&mut self, now_ms: u64) -> Result<Option<DueHint>, PacingError> {
        self.check_observation(now_ms)?;
        match self.state {
            State::Armed { deadline_ms } if now_ms >= deadline_ms => {
                self.state = State::Outstanding;
                Ok(Some(DueHint {
                    observed_ms: now_ms,
                    deadline_ms,
                }))
            }
            State::Armed { .. } | State::Outstanding | State::Paused => Ok(None),
            State::Poisoned => Err(PacingError::Poisoned),
        }
    }

    /// Discard an outstanding hint and suppress scheduling until explicit rearm.
    pub fn pause(&mut self, now_ms: u64) -> Result<(), PacingError> {
        self.check_observation(now_ms)?;
        self.state = State::Paused;
        Ok(())
    }

    /// Reset scheduling after durable commit or explicit resume; missed slots are discarded.
    /// This method verifies no commit, identity, activity or authority.
    pub fn rearm(&mut self, now_ms: u64) -> Result<(), PacingError> {
        self.check_observation(now_ms)?;
        let Some(deadline_ms) = now_ms.checked_add(self.period_ms) else {
            self.state = State::Poisoned;
            return Err(PacingError::Overflow);
        };
        self.state = State::Armed { deadline_ms };
        Ok(())
    }

    fn check_observation(&mut self, now_ms: u64) -> Result<(), PacingError> {
        if matches!(self.state, State::Poisoned) {
            return Err(PacingError::Poisoned);
        }
        if now_ms < self.last_observation_ms {
            self.state = State::Poisoned;
            return Err(PacingError::BackwardsObservation);
        }
        self.last_observation_ms = now_ms;
        Ok(())
    }
}
