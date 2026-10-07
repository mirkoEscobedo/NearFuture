use super::{RECEIPT_PROFILE, ReceiptTarget, SourceMinima};
use crate::PeerError;
use sha2::{Digest, Sha256};
/// Exact received operation context. It never normalizes signed wire minima.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiptOperation {
    pub context_digest: [u8; 32],
    pub target: ReceiptTarget,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub frontier: u64,
    pub minimum: SourceMinima,
}
impl ReceiptOperation {
    pub fn transcript(&self, stage: u8, reply_digest: [u8; 32]) -> Result<[u8; 245], PeerError> {
        self.target.validate()?;
        if !(1..=2).contains(&stage)
            || (stage == 1 && reply_digest != [0; 32])
            || self.client_nonce == [0; 32]
            || self.server_nonce == [0; 32]
        {
            return Err(PeerError::Malformed);
        }
        if self.frontier < self.minimum.membership_revision {
            return Err(PeerError::Policy);
        }
        let mut b = Vec::with_capacity(245);
        b.extend(b"NF-PEER-RECEIPT-2\0");
        b.push(stage);
        b.extend(self.context_digest);
        b.extend(self.target.request.as_bytes());
        b.extend(self.target.operation.as_bytes());
        b.extend(self.target.binding);
        b.extend(RECEIPT_PROFILE.to_le_bytes());
        b.extend(self.client_nonce);
        b.extend(self.server_nonce);
        b.extend(self.frontier.to_le_bytes());
        b.extend(self.minimum.membership_revision.to_le_bytes());
        b.extend(self.minimum.event.0.to_le_bytes());
        b.extend(self.minimum.store_revision.to_le_bytes());
        b.extend(reply_digest);
        b.try_into().map_err(|_| PeerError::Malformed)
    }
    pub fn challenge(&self, stage: u8, reply_digest: [u8; 32]) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.transcript(stage, reply_digest)?).into())
    }
}
