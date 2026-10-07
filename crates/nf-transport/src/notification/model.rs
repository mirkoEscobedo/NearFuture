use nf_contract::identity::{AccountId, DeviceId, OperationId, RequestId};
use nf_identity::model::{DeviceProof, Scope};
pub const PROTOCOL: &str = "/nearfuture/peer/notify/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifyLimits {
    pub frame: u16,
    pub queue_bytes: u32,
    pub queue_items: u16,
    pub rate: u16,
    pub burst: u16,
    pub pending: u16,
}
impl Default for NotifyLimits {
    fn default() -> Self {
        Self {
            frame: 1024,
            queue_bytes: 16384,
            queue_items: 16,
            rate: 8,
            burst: 8,
            pending: 1,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifyContext {
    pub session: [u8; 16],
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifySelector {
    pub request: RequestId,
    pub operation: OperationId,
    pub binding: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotifyBody {
    Hello {
        account: AccountId,
        device: DeviceId,
        nonce: [u8; 32],
        required: u32,
        optional: u32,
        offered: NotifyLimits,
    },
    ServerHello {
        nonce: [u8; 32],
        available: u32,
        selected_caps: u32,
        server_limits: NotifyLimits,
        selected: NotifyLimits,
        proof: DeviceProof,
    },
    ClientProof(DeviceProof),
    Finished(DeviceProof),
    BeginSubscribe {
        subscription: [u8; 16],
        selector: NotifySelector,
        nonce: [u8; 32],
        minimum_membership: u64,
        lifetime: u16,
    },
    SubscribeChallenge {
        subscription: [u8; 16],
        client_nonce: [u8; 32],
        server_nonce: [u8; 32],
        frontier: u64,
        challenge: [u8; 32],
    },
    ProveSubscribe {
        subscription: [u8; 16],
        nonce: [u8; 32],
        proof: DeviceProof,
    },
    Subscribed {
        subscription: [u8; 16],
        selector: NotifySelector,
        first_sequence: u64,
        lifetime: u16,
        proof: DeviceProof,
    },
    Notice {
        subscription: [u8; 16],
        sequence: u64,
        selector: NotifySelector,
        nonce: [u8; 32],
        proof: DeviceProof,
    },
    NoticeAck {
        subscription: [u8; 16],
        sequence: u64,
        notice_digest: [u8; 32],
        proof: DeviceProof,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotifyRecord {
    pub context: NotifyContext,
    pub body: NotifyBody,
}
