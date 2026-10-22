use crate::Rejection;
use alloc::vec::Vec;
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};

pub(super) const MAX_BYTES: usize = 1_048_576;
pub(super) struct Writer {
    output: Vec<u8>,
}
impl Writer {
    pub(super) fn header(scope: &ReplicaDecodeScope<'_>) -> ScopedResult<Self, Rejection> {
        let mut writer = Self { output: Vec::new() };
        writer.fixed(b"NF-CANON-1\0", scope)?;
        writer.fixed(&7u16.to_le_bytes(), scope)?;
        writer.fixed(&1u16.to_le_bytes(), scope)?;
        writer.fixed(&1u16.to_le_bytes(), scope)?;
        Ok(writer)
    }

    pub(super) fn fixed(
        &mut self,
        bytes: &[u8],
        scope: &ReplicaDecodeScope<'_>,
    ) -> ScopedResult<(), Rejection> {
        let end = self
            .output
            .len()
            .checked_add(bytes.len())
            .ok_or(ScopedError::Semantic(Rejection::Limit))?;
        if end > MAX_BYTES {
            return Err(ScopedError::Semantic(Rejection::Limit));
        }
        let copied =
            u64::try_from(bytes.len()).map_err(|_| ScopedError::Semantic(Rejection::Limit))?;
        scope.charge_copied(copied).map_err(ScopedError::Budget)?;
        self.output.extend_from_slice(bytes);
        Ok(())
    }

    pub(super) fn u32(
        &mut self,
        value: u32,
        scope: &ReplicaDecodeScope<'_>,
    ) -> ScopedResult<(), Rejection> {
        self.fixed(&value.to_le_bytes(), scope)
    }
    pub(super) fn u64(
        &mut self,
        value: u64,
        scope: &ReplicaDecodeScope<'_>,
    ) -> ScopedResult<(), Rejection> {
        self.fixed(&value.to_le_bytes(), scope)
    }
    pub(super) fn i64(
        &mut self,
        value: i64,
        scope: &ReplicaDecodeScope<'_>,
    ) -> ScopedResult<(), Rejection> {
        self.fixed(&value.to_le_bytes(), scope)
    }
    pub(super) fn count(
        &mut self,
        count: usize,
        scope: &ReplicaDecodeScope<'_>,
    ) -> ScopedResult<(), Rejection> {
        let count = u32::try_from(count).map_err(|_| ScopedError::Semantic(Rejection::Limit))?;
        scope
            .charge_entries(u64::from(count))
            .map_err(ScopedError::Budget)?;
        self.u32(count, scope)
    }
    pub(super) fn finish(self) -> Vec<u8> {
        self.output
    }
}
