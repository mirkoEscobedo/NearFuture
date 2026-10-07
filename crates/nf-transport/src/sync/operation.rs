use super::{reader::Writer, *};
use crate::PeerError;
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncPurpose {
    Sync,
    Manifest,
    Document,
    Ready,
    End,
    Install,
    GapOrRefused,
}
/// Explicit retained-value data. This object cannot reconstruct absent export custody or create a grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncOperationTranscript {
    pub purpose: SyncPurpose,
    pub neutral_digest: [u8; 32],
    pub request: SyncRequestId,
    pub export: Option<ExportId>,
    pub document_digest: Option<[u8; 32]>,
    pub initiating_digest: [u8; 32],
    pub client_operation_nonce: [u8; 32],
    pub server_operation_nonce: [u8; 32],
    pub membership: MemberStamp,
    pub response_digest: Option<[u8; 32]>,
}
impl SyncOperationTranscript {
    pub fn encode(self) -> Result<[u8; 278], PeerError> {
        if self.request.0 == [0; 16] || self.export.is_some_and(|v| v.0 == [0; 16]) {
            return Err(PeerError::Malformed);
        }
        let client = matches!(self.purpose, SyncPurpose::Sync | SyncPurpose::Document);
        if client == self.response_digest.is_some() {
            return Err(PeerError::Malformed);
        }
        if self.purpose == SyncPurpose::Sync
            && (self.export.is_some() || self.document_digest.is_some())
        {
            return Err(PeerError::Malformed);
        }
        if matches!(
            self.purpose,
            SyncPurpose::Manifest
                | SyncPurpose::Document
                | SyncPurpose::Ready
                | SyncPurpose::End
                | SyncPurpose::Install
        ) && (self.export.is_none() || self.document_digest.is_none())
        {
            return Err(PeerError::Malformed);
        }
        let mut w = Writer {
            bytes: Vec::with_capacity(278),
        };
        w.raw(b"NF-SYNC-OP-1\0");
        w.u8(match self.purpose {
            SyncPurpose::Sync => 1,
            SyncPurpose::Manifest => 2,
            SyncPurpose::Document => 3,
            SyncPurpose::Ready => 4,
            SyncPurpose::End => 5,
            SyncPurpose::Install => 6,
            SyncPurpose::GapOrRefused => 7,
        });
        w.raw(&self.neutral_digest);
        w.raw(&self.request.0);
        w.raw(&self.export.map_or([0; 16], |v| v.0));
        w.raw(&self.document_digest.unwrap_or([0; 32]));
        w.raw(&self.initiating_digest);
        w.raw(&self.client_operation_nonce);
        w.raw(&self.server_operation_nonce);
        w.u64(self.membership.revision);
        w.raw(&self.membership.digest);
        w.raw(&self.response_digest.unwrap_or([0; 32]));
        w.bytes.try_into().map_err(|_| PeerError::Malformed)
    }
    pub fn digest(self) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.encode()?).into())
    }
}
/// Hashes the exact unframed prefix before mandatory DeviceProof297.
pub fn signed_prefix_digest(
    record: &SyncRecord,
    policy: SyncWirePolicy,
) -> Result<[u8; 32], PeerError> {
    Ok(Sha256::digest(signed_prefix(record, policy)?).into())
}
pub fn signed_prefix(record: &SyncRecord, policy: SyncWirePolicy) -> Result<Vec<u8>, PeerError> {
    if !matches!(record.body.kind(), 8 | 9 | 13 | 15 | 16 | 17) {
        return Err(PeerError::Unsupported);
    }
    let mut bytes = encode_body(record, policy)?;
    bytes.truncate(bytes.len().checked_sub(297).ok_or(PeerError::Malformed)?);
    Ok(bytes)
}
