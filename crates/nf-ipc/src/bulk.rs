use crate::{IpcError, LocalPrincipal, SessionConfig, SessionFence};
use nf_contract::{canonical::records::MAX_RECORD_BYTES, identity::OperationId};
use sha2::{Digest, Sha256};
struct Transfer {
    id: OperationId,
    count: u32,
    next: u32,
    total: u64,
    received: u64,
    digest: [u8; 32],
    hash: Sha256,
}
/// Incremental digest only; retains no transfer body. An authenticated endpoint supplies binding.
pub struct BulkReceiver {
    config: SessionConfig,
    principal: LocalPrincipal,
    transfer: Option<Transfer>,
}
pub struct TransferPiece {
    pub transfer: OperationId,
    pub chunk_index: u32,
    pub total_bytes: u64,
    pub data: Vec<u8>,
    pub completion: Option<DigestCertificate>,
}
/// Digest evidence is not semantic admission, authorization or game installation.
pub struct DigestCertificate {
    config: SessionConfig,
    principal: LocalPrincipal,
    id: OperationId,
    total: u64,
    digest: [u8; 32],
}
impl DigestCertificate {
    pub fn transfer(&self) -> OperationId {
        self.id
    }
    pub fn principal(&self) -> LocalPrincipal {
        self.principal
    }
    /// Borrowed digest verification only. A future separately budgeted closed decoder must admit
    /// semantics before any installation; this function never parses a canonical record.
    pub fn verify_bytes<'a>(
        self,
        bytes: &'a [u8],
        fence: &SessionFence,
    ) -> Result<VerifiedTransferBytes<'a>, IpcError> {
        fence.check(self.config.runtime_session)?;
        if bytes.len() > MAX_RECORD_BYTES || bytes.len() as u64 != self.total {
            return Err(IpcError::Limit);
        }
        if Sha256::digest(bytes).as_slice() != self.digest {
            return Err(IpcError::Malformed);
        }
        Ok(VerifiedTransferBytes {
            bytes,
            config: self.config,
            principal: self.principal,
            id: self.id,
            digest: self.digest,
        })
    }
}
/// Opaque borrowed transfer bytes. The type deliberately grants no semantic or installation authority.
pub struct VerifiedTransferBytes<'a> {
    bytes: &'a [u8],
    config: SessionConfig,
    principal: LocalPrincipal,
    id: OperationId,
    digest: [u8; 32],
}
impl<'a> VerifiedTransferBytes<'a> {
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
    pub fn principal(&self) -> LocalPrincipal {
        self.principal
    }
    pub fn transfer(&self) -> OperationId {
        self.id
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
impl BulkReceiver {
    /// This pure value binding does not authenticate token possession. NodeServer does that first.
    pub fn new(config: SessionConfig, principal: LocalPrincipal) -> Result<Self, IpcError> {
        config.validate()?;
        Ok(Self {
            config,
            principal,
            transfer: None,
        })
    }
    pub fn accept(
        &mut self,
        encoded: &[u8],
        fence: &SessionFence,
    ) -> Result<TransferPiece, IpcError> {
        let result = self.accept_inner(encoded, fence);
        if result.is_err() {
            self.transfer = None;
        }
        result
    }
    fn accept_inner(
        &mut self,
        encoded: &[u8],
        fence: &SessionFence,
    ) -> Result<TransferPiece, IpcError> {
        fence.check(self.config.runtime_session)?;
        let limits = &self.config.limits;
        let chunk = nf_wire::decode_chunk_with_limits(
            encoded,
            nf_wire::Limits {
                frame_bytes: limits.control_frame_bytes as usize,
                field_bytes: limits.chunk_bytes as usize,
                collection_items: limits.collection_items as usize,
                depth: limits.nesting_depth as usize,
                decoded_bytes: limits.decoded_bytes as usize,
                ..Default::default()
            },
        )
        .map_err(|e| match e {
            nf_wire::WireError::Limit => IpcError::Limit,
            _ => IpcError::Malformed,
        })?;
        // A 64MiB transport ceiling does not widen the closed 1MiB canonical document limit.
        if chunk.total_bytes > limits.transfer_bytes
            || chunk.total_bytes > MAX_RECORD_BYTES as u64
            || chunk.data.len() > limits.chunk_bytes as usize
        {
            return Err(IpcError::Limit);
        }
        let id = OperationId::from_slice(&chunk.transfer_id.ok_or(IpcError::Malformed)?.value)
            .map_err(|_| IpcError::Malformed)?;
        let digest: [u8; 32] = chunk
            .snapshot_digest
            .ok_or(IpcError::Malformed)?
            .value
            .try_into()
            .map_err(|_| IpcError::Malformed)?;
        let result = self.accept_chunk(
            id,
            digest,
            chunk.chunk_count,
            chunk.chunk_index,
            chunk.total_bytes,
            chunk.data,
        );
        if result.is_err() {
            self.transfer = None;
        }
        result
    }
    fn accept_chunk(
        &mut self,
        id: OperationId,
        digest: [u8; 32],
        count: u32,
        index: u32,
        total: u64,
        data: Vec<u8>,
    ) -> Result<TransferPiece, IpcError> {
        if self.transfer.is_none() {
            if index != 0 {
                return Err(IpcError::Malformed);
            }
            self.transfer = Some(Transfer {
                id,
                count,
                next: 0,
                total,
                received: 0,
                digest,
                hash: Sha256::new(),
            });
        }
        let state = self.transfer.as_mut().ok_or(IpcError::Malformed)?;
        if state.id != id
            || state.digest != digest
            || state.count != count
            || state.total != total
            || state.next != index
        {
            return Err(IpcError::Malformed);
        }
        let received = state
            .received
            .checked_add(data.len() as u64)
            .ok_or(IpcError::Limit)?;
        if received > total {
            return Err(IpcError::Limit);
        }
        state.received = received;
        state.hash.update(&data);
        state.next += 1;
        let completion = if state.next == count {
            let state = self.transfer.take().ok_or(IpcError::Malformed)?;
            if state.received != state.total || state.hash.finalize().as_slice() != state.digest {
                return Err(IpcError::Malformed);
            }
            Some(DigestCertificate {
                config: self.config.clone(),
                principal: self.principal,
                id,
                total,
                digest,
            })
        } else {
            None
        };
        Ok(TransferPiece {
            transfer: id,
            chunk_index: index,
            total_bytes: total,
            data,
            completion,
        })
    }
}
