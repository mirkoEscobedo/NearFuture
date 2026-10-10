use nf_contract::identity::RequestId;
use nf_identity::model::DeviceProof;
use nf_store::chat::{IssuedChallenge, SignedMessage, outbox::SignedChatReceipt};

pub const MAX_FRAME_BYTES: usize = 4096;
pub const PROTOCOL: &str = "/nearfuture/chat/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireContext {
    pub policy_digest: [u8; 32],
    /// Correlation for this exchange; a receipt may retain an earlier original request.
    pub request: RequestId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    Unsupported,
    Unauthorized,
    Limit,
    Replay,
    Conflict,
    Offline,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChatFrame {
    PostChallenge {
        context: WireContext,
        signed: Box<SignedMessage>,
    },
    PostProof {
        context: WireContext,
        signed: Box<SignedMessage>,
        ticket: [u8; 16],
        proof: Box<DeviceProof>,
    },
    Issued {
        context: WireContext,
        challenge: IssuedChallenge,
    },
    Delivered {
        context: WireContext,
        signed: Box<SignedChatReceipt>,
    },
    Refused {
        context: WireContext,
        reason: Refusal,
    },
}
impl ChatFrame {
    pub fn is_request(&self) -> bool {
        matches!(self, Self::PostChallenge { .. } | Self::PostProof { .. })
    }
}
