use super::HostError;
use nf_nex_shadow::Difference;

#[derive(Clone, Copy, Default)]
pub(super) struct Counts {
    pub started: u64,
    pub completed: u64,
    pub unavailable: u64,
    pub matched: u64,
    pub mismatched: u64,
    pub facets: [u64; 6],
    pub failures: u64,
    pub revocations: u64,
}
fn increment(value: u64) -> Result<u64, HostError> {
    value.checked_add(1).ok_or(HostError::Bounds)
}
pub(super) fn index(value: Difference) -> usize {
    match value {
        Difference::Kind => 0,
        Difference::Lifecycle => 1,
        Difference::Priority => 2,
        Difference::Decision => 3,
        Difference::Draws => 4,
        Difference::Effects => 5,
    }
}
impl Counts {
    pub fn reserve_attempt(&self) -> Result<(), HostError> {
        // Reserve room for either complete or unavailable before any evaluation.
        for value in [
            self.started,
            self.completed,
            self.unavailable,
            self.matched,
            self.mismatched,
            self.failures,
            self.revocations,
        ] {
            increment(value)?;
        }
        for value in self.facets {
            increment(value)?;
        }
        Ok(())
    }
    pub fn completed(self, differences: &[Difference]) -> Result<Self, HostError> {
        let mut next = self;
        next.started = increment(next.started)?;
        next.completed = increment(next.completed)?;
        if differences.is_empty() {
            next.matched = increment(next.matched)?;
        } else {
            next.mismatched = increment(next.mismatched)?;
            for difference in differences {
                let count = &mut next.facets[index(*difference)];
                *count = increment(*count)?;
            }
        }
        Ok(next)
    }
    pub fn unavailable(self) -> Result<Self, HostError> {
        let mut next = self;
        next.started = increment(next.started)?;
        next.unavailable = increment(next.unavailable)?;
        next.failures = increment(next.failures)?;
        next.revocations = increment(next.revocations)?;
        Ok(next)
    }
    pub fn revoked(self) -> Result<Self, HostError> {
        Ok(Self {
            revocations: increment(self.revocations)?,
            ..self
        })
    }
}
