use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
    foot::StorageFoot,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub(super) sequence: u64,
    pub(super) operation: [u8; 16],
    pub(super) purpose: u32,
    pub(super) result: u8,
    pub(super) foot: StorageFoot,
    pub(super) action: [u8; 32],
    pub(super) revision: u64,
    pub(super) cancellations: [u8; 32],
}
impl Receipt {
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        if self.sequence == 0
            || self.revision == 0
            || !matches!(self.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
            || !matches!(self.result, 1..=3)
            || (self.result == 3 && (self.purpose != 8 || self.foot.profile != 5))
            || (self.purpose == 8 && self.result == 1)
        {
            return Err(ManagedError::Corrupt);
        }
        let mut w = Encoder::new(b"NF-OWNER-RECEIPT-1\0", 4096)?;
        w.u64(self.sequence)?;
        w.raw(&self.operation)?;
        w.u32(self.purpose)?;
        w.byte(self.result)?;
        w.blob(self.foot.canonical_bytes(), 2048)?;
        w.raw(&self.action)?;
        w.u64(self.revision)?;
        w.raw(&self.cancellations)?;
        Ok(w.finish())
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-RECEIPT-1\0", 4096)?;
        let value = Self {
            sequence: r.u64()?,
            operation: r.fixed()?,
            purpose: r.u32()?,
            result: r.byte()?,
            foot: StorageFoot::decode(r.blob(2048)?)?,
            action: r.fixed()?,
            revision: r.u64()?,
            cancellations: r.fixed()?,
        };
        r.finish()?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
}

impl Receipt {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn operation(&self) -> [u8; 16] {
        self.operation
    }
    pub fn purpose(&self) -> u32 {
        self.purpose
    }
    pub fn result(&self) -> ReceiptResult {
        match self.result {
            1 => ReceiptResult::KnownPost,
            2 => ReceiptResult::KnownPreAborted,
            _ => ReceiptResult::RetiredPostAwaitingSelection,
        }
    }
    pub fn storage_foot(&self) -> &StorageFoot {
        &self.foot
    }
    pub fn outcome_catalog_revision(&self) -> u64 {
        self.revision
    }
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.encode()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptResult {
    KnownPost,
    KnownPreAborted,
    RetiredPostAwaitingSelection,
}
