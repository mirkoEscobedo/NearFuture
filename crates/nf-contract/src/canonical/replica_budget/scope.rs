use super::{
    BudgetResult, Core, ReplicaBudgetError, ReplicaDecodeScope, ReplicaUsage, ScopedError,
    ScopedResult,
};
struct DepthGuard<'a> {
    core: &'a Core,
    previous: u8,
}
impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        let mut usage = self.core.usage.get();
        usage.depth = self.previous;
        self.core.usage.set(usage);
    }
}
impl<'brand> ReplicaDecodeScope<'brand> {
    /// Charges declared logical elements before iteration/growth; accepted work is never refunded.
    pub fn charge_entries(&self, n: u64) -> BudgetResult<()> {
        let mut usage = self.core.usage.get();
        usage.entries = self.core.added(
            usage.entries,
            n,
            u64::from(self.core.limits.entries),
            ReplicaBudgetError::Entries,
        )?;
        self.core.usage.set(usage);
        Ok(())
    }
    /// Charges logical owned copies before growth, excluding allocator headers/rounding.
    pub fn charge_copied(&self, n: u64) -> BudgetResult<()> {
        let mut usage = self.core.usage.get();
        usage.copied_bytes = self.core.added(
            usage.copied_bytes,
            n,
            self.core.limits.copied,
            ReplicaBudgetError::CopiedBytes,
        )?;
        self.core.usage.set(usage);
        Ok(())
    }
    /// Same core/brand, lexical depth restored on normal return, semantic error or unwind.
    pub fn nested<R, E, F>(&self, f: F) -> ScopedResult<R, E>
    where
        F: FnOnce(&ReplicaDecodeScope<'brand>) -> ScopedResult<R, E>,
    {
        self.core.healthy().map_err(ScopedError::Budget)?;
        let mut usage = self.core.usage.get();
        let next = usage
            .depth
            .checked_add(1)
            .ok_or_else(|| ScopedError::Budget(self.core.fail(ReplicaBudgetError::Overflow)))?;
        if next > self.core.limits.depth {
            return Err(ScopedError::Budget(
                self.core.fail(ReplicaBudgetError::Depth),
            ));
        }
        let _depth = DepthGuard {
            core: self.core,
            previous: usage.depth,
        };
        usage.depth = next;
        self.core.usage.set(usage);
        self.core.complete(f(self))
    }
    pub fn usage(&self) -> ReplicaUsage {
        self.core.usage.get()
    }
    pub fn failure(&self) -> Option<ReplicaBudgetError> {
        self.core.failed.get()
    }
}
