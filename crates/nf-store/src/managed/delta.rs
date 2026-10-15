use super::{bytes::{Encoder, Reader, same}, error::{ManagedError, Result},
    maps::{Maps, ReceiptRow}, registry::RegistryRow};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) enum RegistryPurpose { None, Adopt, Stage }
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(super) struct Delta {
    pub registry: BTreeMap<u32, (Option<RegistryRow>, Option<RegistryRow>)>,
    pub receipts: BTreeMap<u64, (Option<ReceiptRow>, Option<ReceiptRow>)>,
}
impl Delta {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.registry.len() > 5 || self.receipts.len() > 2 { return Err(ManagedError::Limit); }
        let mut w = Encoder::new(b"NF-OWNER-ROW-DELTA-1\0", 131_072)?;
        w.u32(self.registry.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        for (kind, (before, after)) in &self.registry {
            if !(1..=5).contains(kind) || before.as_ref().is_some_and(|v| v.kind != *kind)
                || after.as_ref().is_some_and(|v| v.kind != *kind) || before == after
            { return Err(ManagedError::Corrupt); }
            w.u32(*kind)?;
            w.option(before.as_ref(), |w, v| v.write(w))?;
            w.option(after.as_ref(), |w, v| v.write(w))?;
        }
        w.u32(self.receipts.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        for (sequence, (before, after)) in &self.receipts {
            if *sequence == 0 || before.as_ref().is_some_and(|v| v.sequence != *sequence)
                || after.as_ref().is_some_and(|v| v.sequence != *sequence) || before == after
            { return Err(ManagedError::Corrupt); }
            w.u64(*sequence)?;
            w.option(before.as_ref(), |w, v| v.write(w))?;
            w.option(after.as_ref(), |w, v| v.write(w))?;
        }
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-ROW-DELTA-1\0", 131_072)?;
        let mut value = Self::default(); let n = r.u32()?;
        if n > 5 { return Err(ManagedError::Limit); }
        let mut previous = 0;
        for _ in 0..n {
            let kind = r.u32()?;
            if kind <= previous { return Err(ManagedError::Corrupt); }
            previous = kind;
            value.registry.insert(kind, (r.option(RegistryRow::read)?, r.option(RegistryRow::read)?));
        }
        let n = r.u32()?; if n > 2 { return Err(ManagedError::Limit); }
        let mut previous = 0;
        for _ in 0..n {
            let sequence = r.u64()?;
            if sequence <= previous { return Err(ManagedError::Corrupt); }
            previous = sequence;
            value.receipts.insert(sequence, (r.option(ReceiptRow::read)?, r.option(ReceiptRow::read)?));
        }
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
    pub fn additions(rows: Vec<RegistryRow>) -> Result<Self> {
        let mut delta = Self::default();
        for row in rows {
            if delta.registry.insert(row.kind, (None, Some(row))).is_some() { return Err(ManagedError::Corrupt); }
        }
        delta.encode()?; Ok(delta)
    }
    pub fn outcome(old: &Maps, receipt: ReceiptRow) -> Result<Self> {
        let mut delta = Self::default();
        if old.receipts.len() == 256 {
            let (sequence, oldest) = old.receipts.first_key_value().ok_or(ManagedError::Corrupt)?;
            delta.receipts.insert(*sequence, (Some(oldest.clone()), None));
        }
        if delta.receipts.insert(receipt.sequence, (None, Some(receipt))).is_some() { return Err(ManagedError::Replay); }
        delta.encode()?; Ok(delta)
    }
    pub fn apply(&self, old: &Maps, purpose: RegistryPurpose, expected_receipt: Option<&ReceiptRow>) -> Result<Maps> {
        self.encode()?;
        let required: BTreeSet<u32> = match purpose {
            RegistryPurpose::None => BTreeSet::new(),
            RegistryPurpose::Adopt => BTreeSet::from([1, 4, 5]),
            RegistryPurpose::Stage => BTreeSet::from([2, 3]),
        };
        if self.registry.keys().copied().collect::<BTreeSet<_>>() != required { return Err(ManagedError::Corrupt); }
        let mut next = old.clone();
        for (kind, (before, after)) in &self.registry {
            if before.is_some() || old.registry.contains_key(kind) { return Err(ManagedError::OperationConflict); }
            next.registry.insert(*kind, after.clone().ok_or(ManagedError::Corrupt)?);
        }
        match expected_receipt {
            None if !self.receipts.is_empty() => return Err(ManagedError::Corrupt),
            None => {},
            Some(receipt) => {
                if old.receipts.contains_key(&receipt.sequence)
                    || old.receipts.last_key_value().is_some_and(|(last, _)| *last >= receipt.sequence)
                    || old.receipts.values().any(|r| r.operation == receipt.operation)
                { return Err(ManagedError::Replay); }
                let mut exact = BTreeMap::from([(receipt.sequence, (None, Some(receipt.clone())))]);
                if old.receipts.len() == 256 {
                    let (sequence, oldest) = old.receipts.first_key_value().ok_or(ManagedError::Corrupt)?;
                    exact.insert(*sequence, (Some(oldest.clone()), None));
                }
                if self.receipts != exact { return Err(ManagedError::Corrupt); }
            },
        }
        for (sequence, (before, after)) in &self.receipts {
            if old.receipts.get(sequence) != before.as_ref() { return Err(ManagedError::Corrupt); }
            match after {
                Some(value) => { next.receipts.insert(*sequence, value.clone()); },
                None => { next.receipts.remove(sequence); },
            }
        }
        next.encode()?; Ok(next)
    }
}
