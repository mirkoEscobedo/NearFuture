use super::*;
use crate::records::PeerContext;
use nf_contract::identity::{AccountId, DeviceId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncLane {
    Control,
    Transfer,
}
impl SyncLane {
    pub fn protocol(self) -> &'static str {
        match self {
            Self::Control => "/nearfuture/peer/sync/1",
            Self::Transfer => "/nearfuture/peer/sync-transfer/1",
        }
    }
}
/// Literal expected data only; not an implementation registry or binary attestation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpectedProfilePins {
    pub implementation: [u8; 32],
    pub schema: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncHello {
    pub account: AccountId,
    pub device: DeviceId,
    pub nonce: [u8; 32],
    pub required: u32,
    pub optional: u32,
    pub offered: super::SyncLimits,
    pub pins: ExpectedProfilePins,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncBody {
    Hello(SyncHello),
    ServerHello(ServerHello),
    ClientProof(nf_identity::model::DeviceProof),
    Finished(nf_identity::model::DeviceProof),
    BeginSync(BeginSync),
    SyncChallenge(SyncChallenge),
    ProveSync(ProveSync),
    ManifestOffer(ManifestOffer),
    GapRequiresSnapshot(GapRequiresSnapshot),
    BeginDocument(BeginDocument),
    DocumentChallenge(DocumentChallenge),
    ProveDocument(ProveDocument),
    DocumentReady(DocumentReady),
    SyncChunk(SyncChunk),
    DocumentEnd(DocumentEnd),
    ReplicaInstallReceipt(ReplicaInstallReceipt),
    SyncRefused(SyncRefused),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRecord {
    pub lane: SyncLane,
    pub context: PeerContext,
    pub body: SyncBody,
}
