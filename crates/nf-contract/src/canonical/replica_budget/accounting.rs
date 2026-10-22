use super::{BudgetResult, Core, ReplicaBudgetError, ScopedError, ScopedResult};
impl Core {
    pub(super) fn healthy(&self) -> BudgetResult<()> {
        self.failed.get().map_or(Ok(()), Err)
    }
    pub(super) fn fail(&self, error: ReplicaBudgetError) -> ReplicaBudgetError {
        let first = self.failed.get().unwrap_or(error);
        self.failed.set(Some(first));
        first
    }
    pub(super) fn complete<R, E>(&self, result: ScopedResult<R, E>) -> ScopedResult<R, E> {
        if let Err(ScopedError::Budget(error)) = &result {
            self.fail(*error);
        }
        match self.failed.get() {
            Some(error) => Err(ScopedError::Budget(error)),
            None => result,
        }
    }
    pub(super) fn added(
        &self,
        old: u64,
        n: u64,
        limit: u64,
        error: ReplicaBudgetError,
    ) -> BudgetResult<u64> {
        self.healthy()?;
        let next = old
            .checked_add(n)
            .ok_or_else(|| self.fail(ReplicaBudgetError::Overflow))?;
        if next > limit {
            return Err(self.fail(error));
        }
        Ok(next)
    }
}
