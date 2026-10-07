use crate::PeerError;
use nf_contract::identity::RequestId;
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug)]
pub struct QueryTranscript {
    pub context_digest: [u8; 32],
    pub request: RequestId,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub frontier: u64,
    pub minimum_membership: u64,
}
impl QueryTranscript {
    pub fn challenge(self, stage: u8, reply_digest: [u8; 32]) -> Result<[u8; 32], PeerError> {
        if !(1..=2).contains(&stage) || stage == 1 && reply_digest != [0; 32] {
            return Err(PeerError::Malformed);
        }
        if self.frontier < self.minimum_membership {
            return Err(PeerError::Unauthorized);
        }
        let mut b = Vec::with_capacity(177);
        b.extend(b"NF-PEER-QUERY-1\0");
        b.push(stage);
        b.extend(self.context_digest);
        b.extend(self.request.as_bytes());
        b.extend(self.client_nonce);
        b.extend(self.server_nonce);
        b.extend(self.frontier.to_le_bytes());
        b.extend(self.minimum_membership.to_le_bytes());
        b.extend(reply_digest);
        if b.len() != 177 {
            return Err(PeerError::Malformed);
        }
        Ok(Sha256::digest(b).into())
    }
}
