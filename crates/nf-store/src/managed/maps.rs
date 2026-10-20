use super::{
    bytes::{Encoder, Reader, hash, same},
    error::{ManagedError, Result},
    receipt::Receipt,
    registry::RegistryRow,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ReceiptRow {
    pub sequence: u64,
    pub operation: [u8; 16],
    pub result: Vec<u8>,
    pub digest: [u8; 32],
}
impl ReceiptRow {
    pub fn new(receipt: &Receipt) -> Result<Self> {
        let result = receipt.encode()?;
        Ok(Self {
            sequence: receipt.sequence,
            operation: receipt.operation,
            digest: hash(&result),
            result,
        })
    }
    pub fn write(&self, w: &mut Encoder) -> Result<()> {
        let receipt = Receipt::decode(&self.result)?;
        if receipt.sequence != self.sequence
            || receipt.operation != self.operation
            || hash(&self.result) != self.digest
        {
            return Err(ManagedError::Corrupt);
        }
        w.u64(self.sequence)?;
        w.raw(&self.operation)?;
        w.blob(&self.result, 4096)?;
        w.raw(&self.digest)
    }
    pub fn read(r: &mut Reader<'_>) -> Result<Self> {
        let value = Self {
            sequence: r.u64()?,
            operation: r.fixed()?,
            result: r.blob(4096)?.to_vec(),
            digest: r.fixed()?,
        };
        let mut w = Encoder::plain(4156);
        value.write(&mut w)?;
        Ok(value)
    }
    pub fn cells(&self) -> Vec<super::rows::Cell> {
        use super::rows::Cell::Blob;
        vec![
            Blob(self.sequence.to_be_bytes().to_vec()),
            Blob(self.operation.to_vec()),
            Blob(self.result.clone()),
            Blob(self.digest.to_vec()),
        ]
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(super) struct Maps {
    pub registry: BTreeMap<u32, RegistryRow>,
    pub receipts: BTreeMap<u64, ReceiptRow>,
}
impl Maps {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.registry.len() > 5 || self.receipts.len() > 256 {
            return Err(ManagedError::Limit);
        }
        let mut w = Encoder::new(b"NF-OWNER-CATALOG-ROWS-1\0", 1_179_648)?;
        w.u32(
            self.registry
                .len()
                .try_into()
                .map_err(|_| ManagedError::Limit)?,
        )?;
        let mut artifacts = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        for (kind, row) in &self.registry {
            if *kind != row.kind || !artifacts.insert(row.artifact) || !paths.insert(&row.path) {
                return Err(ManagedError::Corrupt);
            }
            row.write(&mut w)?;
        }
        w.u32(
            self.receipts
                .len()
                .try_into()
                .map_err(|_| ManagedError::Limit)?,
        )?;
        let mut operations = std::collections::BTreeSet::new();
        for (sequence, row) in &self.receipts {
            if *sequence != row.sequence || !operations.insert(row.operation) {
                return Err(ManagedError::Corrupt);
            }
            row.write(&mut w)?;
        }
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-CATALOG-ROWS-1\0", 1_179_648)?;
        let mut value = Self::default();
        let count = r.u32()?;
        if count > 5 {
            return Err(ManagedError::Limit);
        }
        let mut previous = 0;
        for _ in 0..count {
            let row = RegistryRow::read(&mut r)?;
            if row.kind <= previous {
                return Err(ManagedError::Corrupt);
            }
            previous = row.kind;
            value.registry.insert(row.kind, row);
        }
        let count = r.u32()?;
        if count > 256 {
            return Err(ManagedError::Limit);
        }
        let mut previous = 0;
        for _ in 0..count {
            let row = ReceiptRow::read(&mut r)?;
            if row.sequence <= previous {
                return Err(ManagedError::Corrupt);
            }
            previous = row.sequence;
            value.receipts.insert(row.sequence, row);
        }
        r.finish()?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
    pub fn digests(&self) -> Result<([u8; 32], [u8; 32])> {
        self.encode()?;
        let registry = self
            .registry
            .values()
            .map(RegistryRow::cells)
            .collect::<Vec<_>>();
        let receipts = self
            .receipts
            .values()
            .map(ReceiptRow::cells)
            .collect::<Vec<_>>();
        Ok((
            super::rows::value_digest(super::rows::table("artifact_registry")?, &registry)?,
            super::rows::value_digest(super::rows::table("operation_receipts")?, &receipts)?,
        ))
    }
}
