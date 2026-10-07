use super::*;
use crate::{PeerError, records::PeerLimits};
use nf_contract::identity::*;
use nf_identity::model::DeviceProof;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiptTarget {
    pub request: RequestId,
    pub operation: OperationId,
    pub binding: [u8; 32],
}
impl ReceiptTarget {
    pub fn validate(self) -> Result<(), PeerError> {
        if self.request.as_bytes() == &[0; 16]
            || self.operation.as_bytes() == &[0; 16]
            || self.binding == [0; 32]
        {
            Err(PeerError::Malformed)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptUnsupportedReason {
    BindingConflict = 1,
    SourceBelowKnownMinima = 2,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptBody {
    Hello {
        account: AccountId,
        device: DeviceId,
        nonce: [u8; 32],
        required: u32,
        optional: u32,
        offered: PeerLimits,
    },
    ServerHello {
        nonce: [u8; 32],
        available: u32,
        selected_caps: u32,
        server_limits: PeerLimits,
        selected: PeerLimits,
        proof: DeviceProof,
    },
    ClientProof(DeviceProof),
    Finished(DeviceProof),
    Begin {
        target: ReceiptTarget,
        nonce: [u8; 32],
        minimum: SourceMinima,
    },
    Challenge {
        target: ReceiptTarget,
        client_nonce: [u8; 32],
        server_nonce: [u8; 32],
        frontier: u64,
        challenge: [u8; 32],
    },
    Prove {
        request: RequestId,
        nonce: [u8; 32],
        proof: DeviceProof,
    },
    Status {
        status: ReceiptStatus,
        proof: DeviceProof,
    },
    Unsupported {
        request: RequestId,
        reason: ReceiptUnsupportedReason,
        current: SourceMinima,
        proof: DeviceProof,
    },
}
impl ReceiptBody {
    pub fn kind(&self) -> u8 {
        match self {
            Self::Hello { .. } => 1,
            Self::ServerHello { .. } => 2,
            Self::ClientProof(_) => 3,
            Self::Finished(_) => 4,
            Self::Begin { .. } => 5,
            Self::Challenge { .. } => 6,
            Self::Prove { .. } => 7,
            Self::Status { .. } => 8,
            Self::Unsupported { .. } => 9,
        }
    }
}
