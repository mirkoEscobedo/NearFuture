use super::super::HostError;

const CAPACITY: usize = 64;

/// Private policy over actual completed samples; it has no clock or public ingress.
pub(crate) struct Window {
    samples: [u64; CAPACITY],
    length: usize,
    next: usize,
    completed: u64,
}

pub(crate) struct Summary {
    pub completed: u64,
    pub retained: u64,
    pub evicted: u64,
    pub quantiles: [Option<u64>; 4],
}

impl Window {
    pub(crate) fn new() -> Self {
        Self {
            samples: [0; CAPACITY],
            length: 0,
            next: 0,
            completed: 0,
        }
    }

    pub(crate) fn validate(&self, expected_completed: u64) -> Result<(), HostError> {
        let retained = usize::try_from(self.completed.min(64)).map_err(|_| HostError::Bounds)?;
        let position = usize::try_from(self.completed % 64).map_err(|_| HostError::Bounds)?;
        if self.completed != expected_completed
            || self.length != retained
            || self.length > CAPACITY
            || self.next != position
            || self.next >= CAPACITY
        {
            return Err(HostError::Bounds);
        }
        Ok(())
    }

    pub(crate) fn preflight(&self, expected_completed: u64) -> Result<(), HostError> {
        self.validate(expected_completed)?;
        self.completed.checked_add(1).ok_or(HostError::Bounds)?;
        Ok(())
    }

    pub(crate) fn append(
        &mut self,
        nanos: u64,
        previous_completed: u64,
        next_completed: u64,
    ) -> Result<(), HostError> {
        self.preflight(previous_completed)?;
        let completed = self.completed.checked_add(1).ok_or(HostError::Bounds)?;
        if completed != next_completed {
            return Err(HostError::Bounds);
        }
        let next = self.next.checked_add(1).ok_or(HostError::Bounds)? % CAPACITY;
        let length = self
            .length
            .checked_add(1)
            .ok_or(HostError::Bounds)?
            .min(CAPACITY);
        let slot = self.samples.get_mut(self.next).ok_or(HostError::Bounds)?;
        // Every fallible check precedes mutation; this replaces exactly the oldest slot.
        *slot = nanos;
        self.next = next;
        self.length = length;
        self.completed = completed;
        Ok(())
    }

    pub(crate) fn summary(&self, expected_completed: u64) -> Result<Summary, HostError> {
        self.validate(expected_completed)?;
        let retained = u64::try_from(self.length).map_err(|_| HostError::Bounds)?;
        let evicted = self
            .completed
            .checked_sub(retained)
            .ok_or(HostError::Bounds)?;
        let mut quantiles = [None; 4];
        if self.length != 0 {
            let mut sorted = [0; CAPACITY];
            let source = self.samples.get(..self.length).ok_or(HostError::Bounds)?;
            let scratch = sorted.get_mut(..self.length).ok_or(HostError::Bounds)?;
            scratch.copy_from_slice(source);
            scratch.sort_unstable();
            for (value, percentile) in quantiles.iter_mut().zip([500_u64, 950, 990, 999]) {
                let rank = retained
                    .checked_mul(percentile)
                    .and_then(|n| n.checked_add(999))
                    .ok_or(HostError::Bounds)?
                    / 1000;
                if rank == 0 || rank > retained {
                    return Err(HostError::Bounds);
                }
                let index = usize::try_from(rank.checked_sub(1).ok_or(HostError::Bounds)?)
                    .map_err(|_| HostError::Bounds)?;
                *value = Some(*scratch.get(index).ok_or(HostError::Bounds)?);
            }
        }
        Ok(Summary {
            completed: self.completed,
            retained,
            evicted,
            quantiles,
        })
    }
}
