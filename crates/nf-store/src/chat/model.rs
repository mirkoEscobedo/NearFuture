use nf_contract::identity::{AccountId, DeviceId, RequestId};
use nf_identity::model::{DeviceProof, Scope};
pub const MAX_TEXT_BYTES: usize = 2048;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    General,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Author {
    pub account: AccountId,
    pub device: DeviceId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessage {
    pub scope: Scope,
    pub channel: Channel,
    pub author: Author,
    pub message: [u8; 16],
    /// Sequence belongs to this account/device stream, never to all senders.
    pub sequence: u64,
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedMessage {
    pub message: ChatMessage,
    pub signature: [u8; 64],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChatPolicy {
    pub scope: Scope,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChatReceipt {
    pub message: [u8; 16],
    pub author: Author,
    pub source_sequence: u64,
    /// Local pagination position, not a globally authoritative message order.
    pub receiver_cursor: u64,
    pub original_request: RequestId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryQuery {
    pub request: RequestId,
    pub reader: Author,
    pub channel: Channel,
    pub after_cursor: u64,
    pub limit: u16,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub receiver_cursor: u64,
    pub signed: SignedMessage,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_cursor: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownChatFrontiers {
    pub scope: Scope,
    pub revision: u64,
    pub membership_revision: u64,
}
pub enum ChallengeRequest<'a> {
    Post {
        request: RequestId,
        message: &'a SignedMessage,
    },
    History(&'a HistoryQuery),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IssuedChallenge {
    pub ticket: [u8; 16],
    pub challenge: [u8; 32],
    pub membership_revision: u64,
}
/// `peer` is supplied by the authenticated transport boundary, never inferred from proof bytes.
pub struct ProofAttempt<'a> {
    pub ticket: [u8; 16],
    pub proof: &'a DeviceProof,
    pub peer: &'a [u8],
}
