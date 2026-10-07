use crate::{PeerError, records::BulkDescriptor};
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug)]
pub struct BulkTranscript {
    pub context_digest: [u8; 32],
    pub descriptor: BulkDescriptor,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub frontier: u64,
    pub minimum_membership: u64,
}
impl BulkTranscript {
    pub fn challenge(self, stage: u8, reply_digest: [u8; 32]) -> Result<[u8; 32], PeerError> {
        if !(1..=2).contains(&stage) || stage == 1 && reply_digest != [0; 32] {
            return Err(PeerError::Malformed);
        }
        if self.frontier < self.minimum_membership {
            return Err(PeerError::Unauthorized);
        }
        let mut h = Sha256::new();
        h.update(b"NF-PEER-BULK-1\0");
        h.update([stage]);
        h.update(self.context_digest);
        h.update(self.descriptor.transfer);
        h.update(self.client_nonce);
        h.update(self.server_nonce);
        h.update(self.frontier.to_le_bytes());
        h.update(self.minimum_membership.to_le_bytes());
        h.update(self.descriptor.digest());
        h.update(reply_digest);
        Ok(h.finalize().into())
    }
}
