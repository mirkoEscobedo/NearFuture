use super::ReceiptBody;
use crate::{PeerError, auth::ServerPin, records::PeerContext};
use nf_contract::{canonical::binding::RequestBinding, identity::*};

pub const HEADER_BYTES: usize = 126;
pub const MAX_BODY_BYTES: usize = 556;
pub const RECEIPT_PROFILE: u16 = 1;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMinima {
    pub event: EventSeq,
    pub store_revision: u64,
    pub membership_revision: u64,
}
impl SourceMinima {
    pub fn admits(self, known: Self) -> bool {
        self.event >= known.event
            && self.store_revision >= known.store_revision
            && self.membership_revision >= known.membership_revision
    }
}
/// Immutable trusted original data; constructing it grants no authority.
#[derive(Clone, Debug)]
pub struct OriginalReceipt {
    operation: OperationId,
    original: RequestBinding,
    source: ServerPin,
}
impl OriginalReceipt {
    pub fn new(
        operation: OperationId,
        original: RequestBinding,
        source: ServerPin,
    ) -> Result<Self, PeerError> {
        if operation.as_bytes() == &[0; 16]
            || original.request_id.as_bytes() == &[0; 16]
            || original.account_id.as_bytes() == &[0; 16]
            || original.device_id.as_bytes() == &[0; 16]
            || original.universe_id.as_bytes() == &[0; 16]
            || original.history_id.as_bytes() == &[0; 16]
            || source.account.as_bytes() == &[0; 16]
            || source.device.as_bytes() == &[0; 16]
            || !(1..=3).contains(&original.operation_kind)
        {
            return Err(PeerError::Malformed);
        }
        Ok(Self {
            operation,
            original,
            source,
        })
    }
    pub fn request(&self) -> RequestId {
        self.original.request_id
    }
    pub fn operation(&self) -> OperationId {
        self.operation
    }
    pub fn original(&self) -> &RequestBinding {
        &self.original
    }
    pub fn binding_digest(&self) -> [u8; 32] {
        self.original.digest()
    }
    pub fn source(&self) -> ServerPin {
        self.source
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptPhase {
    Unknown,
    Pending,
    Rejected { sequence: EventSeq },
    Committed { sequence: EventSeq },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptStatus {
    pub request: RequestId,
    pub account: AccountId,
    pub device: DeviceId,
    pub operation: OperationId,
    pub binding: [u8; 32],
    pub phase: ReceiptPhase,
    pub current: SourceMinima,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptRecord {
    pub context: PeerContext,
    pub body: ReceiptBody,
}
