#![doc = include_str!("replica_budget_api.md")]
//! Logical cumulative accounting; callers charge before allocating or copying.
use core::{cell::Cell, marker::PhantomData};
#[path = "replica_budget/accounting.rs"]
mod accounting;
#[path = "replica_budget/scope.rs"]
mod scope;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplicaBudgetError {
    Depth,
    Entries,
    CopiedBytes,
    Overflow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopedError<E> {
    Budget(ReplicaBudgetError),
    Semantic(E),
}
pub type BudgetResult<T> = Result<T, ReplicaBudgetError>;
pub type ScopedResult<T, E> = Result<T, ScopedError<E>>;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplicaDecodeLimits {
    depth: u8,
    entries: u32,
    copied: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplicaUsage {
    pub entries: u64,
    pub copied_bytes: u64,
    pub depth: u8,
}
struct Core {
    limits: ReplicaDecodeLimits,
    usage: Cell<ReplicaUsage>,
    failed: Cell<Option<ReplicaBudgetError>>,
}
/// An invariant branded shared scope. Counters cannot be reset or replaced by consumers.
pub struct ReplicaDecodeScope<'brand> {
    core: &'brand Core,
    brand: PhantomData<fn(&'brand mut ()) -> &'brand mut ()>,
}
impl ReplicaDecodeLimits {
    /// Depth includes the root (one); zero entry/copy caps permit zero charges only.
    pub fn new(depth: u8, entries: u32, copied: u64) -> BudgetResult<Self> {
        if !(1..=32).contains(&depth) {
            return Err(ReplicaBudgetError::Depth);
        }
        if entries > 32768 {
            return Err(ReplicaBudgetError::Entries);
        }
        if copied > 4194304 {
            return Err(ReplicaBudgetError::CopiedBytes);
        }
        Ok(Self {
            depth,
            entries,
            copied,
        })
    }
    /// Creates one lexical document scope; a budget failure remains fatal if caught inside.
    pub fn with_scope<R, E, F>(self, f: F) -> ScopedResult<R, E>
    where
        F: for<'brand> FnOnce(&ReplicaDecodeScope<'brand>) -> ScopedResult<R, E>,
    {
        let core = Core {
            limits: self,
            usage: Cell::new(ReplicaUsage {
                entries: 0,
                copied_bytes: 0,
                depth: 1,
            }),
            failed: Cell::new(None),
        };
        let scope = ReplicaDecodeScope {
            core: &core,
            brand: PhantomData,
        };
        core.complete(f(&scope))
    }
}
