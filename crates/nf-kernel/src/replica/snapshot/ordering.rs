use crate::Rejection;
use alloc::vec::Vec;
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};

pub(super) fn rows<'a, T, K: Ord>(
    values: &'a [T],
    key: impl Fn(&T) -> K,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Vec<&'a T>, Rejection> {
    let mut ordered = Vec::new();
    for value in values {
        scope.nested(|row| {
            row.charge_entries(1).map_err(ScopedError::Budget)?;
            row.charge_copied(8).map_err(ScopedError::Budget)?;
            ordered.push(value);
            Ok(())
        })?;
    }
    // Borrowed valid World guarantees unique entity/provider/manifest/ownership keys.
    // Therefore unstable sorting has the same canonical order without heap scratch.
    ordered.sort_unstable_by_key(|value| key(*value));
    Ok(ordered)
}
