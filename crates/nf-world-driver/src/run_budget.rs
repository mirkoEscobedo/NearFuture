use crate::{Driver, DriverError};
use nf_store::{Boundary, miniature::MiniatureStoreError};
use std::time::Instant;
impl Driver {
    pub(super) fn check_budget(&self) -> Result<(), DriverError> {
        if self
            .run_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            Err(DriverError::WindowExpired)
        } else {
            Ok(())
        }
    }
    pub(super) fn budget_expired(&self) -> bool {
        self.run_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}
/// A rejection-only cut; this never substitutes for Store authorization or produces an acknowledgement.
pub(super) fn deadline_hook(
    deadline: Option<Instant>,
) -> impl FnMut(Boundary) -> Result<(), MiniatureStoreError> {
    move |boundary| {
        if matches!(
            boundary,
            Boundary::BeforeTransaction | Boundary::BeforeCommit
        ) && deadline.is_some_and(|end| Instant::now() >= end)
        {
            Err(MiniatureStoreError::Expired)
        } else {
            Ok(())
        }
    }
}
