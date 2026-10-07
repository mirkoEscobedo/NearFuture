use super::*;
use nf_identity::model::DeviceProof;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Point {
    pub store_revision: u64,
    pub event: nf_contract::identity::EventSeq,
    pub world: [u8; 32],
    pub state: [u8; 32],
    pub journal: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemberStamp {
    pub revision: u64,
    pub digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncRequestId(pub [u8; 16]);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportId(pub [u8; 16]);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferId(pub [u8; 16]);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncMode {
    Delta,
    Snapshot,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GapReason {
    PrefixPruned,
    BaseMismatch,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalReason {
    ExportPreparing,
    UnsupportedBase,
    SourceBelowMinima,
    Capacity,
    ExportExpired,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerHello {
    pub nonce: [u8; 32],
    pub available: u32,
    pub selected_caps: u32,
    pub offered: SyncLimits,
    pub selected: SyncLimits,
    pub membership_digest: [u8; 32],
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginSync {
    pub mode: SyncMode,
    pub pins: ExpectedProfilePins,
    pub client_nonce: [u8; 32],
    pub minimum_event: nf_contract::identity::EventSeq,
    pub minimum_store: u64,
    pub minimum_membership: u64,
    pub base: Option<Point>,
    pub membership_digest: [u8; 32],
    pub request: SyncRequestId,
    pub maximum_documents: u16,
    pub maximum_data_bytes: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncChallenge {
    pub request: SyncRequestId,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub captured: Point,
    pub membership: MemberStamp,
    pub challenge: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProveSync {
    pub request: SyncRequestId,
    pub client_nonce: [u8; 32],
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestOffer {
    pub request: SyncRequestId,
    pub export: ExportId,
    pub target: Point,
    pub manifest_digest: [u8; 32],
    pub manifest_length: u32,
    pub document_count: u16,
    pub data_bytes: u64,
    pub current: Point,
    pub membership: MemberStamp,
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GapRequiresSnapshot {
    pub request: SyncRequestId,
    pub reason: GapReason,
    pub oldest: Option<Point>,
    pub current: Point,
    pub membership: MemberStamp,
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeginDocument {
    pub transfer: TransferId,
    pub export: ExportId,
    pub document_digest: [u8; 32],
    pub total: u32,
    pub count: u16,
    pub client_nonce: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentChallenge {
    pub transfer: TransferId,
    pub export: ExportId,
    pub document_digest: [u8; 32],
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub membership: MemberStamp,
    pub challenge: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProveDocument {
    pub transfer: TransferId,
    pub client_nonce: [u8; 32],
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentReady {
    pub transfer: TransferId,
    pub export: ExportId,
    pub document_digest: [u8; 32],
    pub total: u32,
    pub count: u16,
    pub current: Point,
    pub membership: MemberStamp,
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncChunk {
    pub transfer: TransferId,
    pub export: ExportId,
    pub document_digest: [u8; 32],
    pub index: u16,
    pub count: u16,
    pub total: u32,
    pub data: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentEnd {
    pub transfer: TransferId,
    pub export: ExportId,
    pub document_digest: [u8; 32],
    pub received: u32,
    pub assembled_digest: [u8; 32],
    pub current: Point,
    pub membership: MemberStamp,
    pub completion_nonce: [u8; 32],
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplicaInstallReceipt {
    pub export: ExportId,
    pub manifest_digest: [u8; 32],
    pub target: Point,
    pub generation: u64,
    pub proof: DeviceProof,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRefused {
    pub request: SyncRequestId,
    pub export: Option<ExportId>,
    pub reason: RefusalReason,
    pub current: Point,
    pub membership: MemberStamp,
    pub proof: DeviceProof,
}
