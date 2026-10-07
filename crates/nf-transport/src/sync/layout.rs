use super::*;
use crate::PeerError;
impl SyncBody {
    pub fn kind(&self) -> u8 {
        match self {
            Self::Hello(_) => 1,
            Self::ServerHello(_) => 2,
            Self::ClientProof(_) => 3,
            Self::Finished(_) => 4,
            Self::BeginSync(_) => 5,
            Self::SyncChallenge(_) => 6,
            Self::ProveSync(_) => 7,
            Self::ManifestOffer(_) => 8,
            Self::GapRequiresSnapshot(_) => 9,
            Self::BeginDocument(_) => 10,
            Self::DocumentChallenge(_) => 11,
            Self::ProveDocument(_) => 12,
            Self::DocumentReady(_) => 13,
            Self::SyncChunk(_) => 14,
            Self::DocumentEnd(_) => 15,
            Self::ReplicaInstallReceipt(_) => 16,
            Self::SyncRefused(_) => 17,
        }
    }
    pub(super) fn bytes(&self) -> Result<usize, PeerError> {
        Ok(match self {
            Self::Hello(_) => 165,
            Self::ServerHello(_) => 421,
            Self::ClientProof(_) | Self::Finished(_) => 297,
            Self::BeginSync(_) => 294,
            Self::SyncChallenge(_) => 264,
            Self::ProveSync(_) => 345,
            Self::ManifestOffer(_) => 639,
            Self::GapRequiresSnapshot(_) => 579,
            Self::BeginDocument(_) => 102,
            Self::DocumentChallenge(_) => 200,
            Self::ProveDocument(_) => 345,
            Self::DocumentReady(_) => 519,
            Self::SyncChunk(v) => 74usize.checked_add(v.data.len()).ok_or(PeerError::Limit)?,
            Self::DocumentEnd(_) => 581,
            Self::ReplicaInstallReceipt(_) => 465,
            Self::SyncRefused(_) => 482,
        })
    }
}
