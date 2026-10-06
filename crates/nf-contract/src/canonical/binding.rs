//! Retry binding validation, not authorization or durable request storage.
use super::PREFIX;
use crate::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestBinding {
    pub request_id: RequestId,
    pub account_id: AccountId,
    pub device_id: DeviceId,
    pub universe_id: UniverseId,
    pub history_id: HistoryId,
    pub operation_kind: u32,
    pub payload_digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestConflict;
impl RequestBinding {
    pub fn canonical_bytes(&self) -> [u8; 133] {
        let mut output = [0; 133];
        output[..11].copy_from_slice(PREFIX);
        output[11..17].copy_from_slice(&[1, 0, 1, 0, 1, 0]);
        for (i, id) in [
            self.request_id.as_bytes(),
            self.account_id.as_bytes(),
            self.device_id.as_bytes(),
            self.universe_id.as_bytes(),
            self.history_id.as_bytes(),
        ]
        .iter()
        .enumerate()
        {
            output[17 + i * 16..33 + i * 16].copy_from_slice(*id);
        }
        output[97..101].copy_from_slice(&self.operation_kind.to_le_bytes());
        output[101..].copy_from_slice(&self.payload_digest);
        output
    }
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes()).into()
    }
    /// Call after looking up (HistoryId, RequestId) and authenticating the principal.
    /// Both binding and full-intent digest must agree before returning a prior outcome.
    pub fn verify_retry(
        &self,
        candidate: &Self,
        stored_intent: [u8; 32],
        proposed_intent: [u8; 32],
    ) -> Result<(), RequestConflict> {
        if self != candidate || stored_intent != proposed_intent {
            Err(RequestConflict)
        } else {
            Ok(())
        }
    }
}
