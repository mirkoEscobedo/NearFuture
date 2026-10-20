use super::super::{CurrentRead, RemotePrincipal};
use crate::{
    PeerError,
    receipt::{OriginalReceipt, ReceiptPhase, ReceiptStatus, ReceiptTarget, SourceMinima},
};
use nf_identity::{
    model::{DeviceProof, ProtectedOperation},
    signing::device_digest,
};
use std::time::{Duration, Instant};
pub(in crate::receipt_effects) fn fresh(created: Instant) -> Result<(), PeerError> {
    if created.elapsed() >= Duration::from_secs(5) {
        Err(PeerError::Replay)
    } else {
        Ok(())
    }
}
pub(super) fn nonce() -> Result<[u8; 32], PeerError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| PeerError::Entropy)?;
    if bytes == [0; 32] {
        Err(PeerError::Entropy)
    } else {
        Ok(bytes)
    }
}
pub(super) fn principal(c: &crate::auth::HandshakeContext) -> RemotePrincipal {
    RemotePrincipal {
        account: c.client_account,
        device: c.client_device,
        scope: c.context.scope,
        peer: c.client_peer,
    }
}
pub(super) fn target(o: &OriginalReceipt) -> ReceiptTarget {
    ReceiptTarget {
        request: o.request(),
        operation: o.operation(),
        binding: o.binding_digest(),
    }
}
pub(super) fn sign(
    cut: &CurrentRead<'_>,
    challenge: [u8; 32],
    minimum: u64,
) -> Result<DeviceProof, PeerError> {
    let mut proof = DeviceProof {
        scope: cut.membership.scope,
        account: cut.local.account,
        device: cut.local.device,
        frontier: cut.membership.revision,
        peer: cut.local.peer.clone(),
        challenge,
        signature: [0; 64],
    };
    proof.signature = cut
        .device_key
        .sign(&device_digest(&proof).map_err(|_| PeerError::Unauthorized)?);
    verify(cut, &proof, &cut.local.peer, &challenge, minimum)?;
    Ok(proof)
}
pub(super) fn verify(
    cut: &CurrentRead<'_>,
    proof: &DeviceProof,
    peer: &[u8],
    challenge: &[u8; 32],
    minimum: u64,
) -> Result<(), PeerError> {
    cut.membership
        .authorize(
            proof,
            peer,
            challenge,
            minimum,
            ProtectedOperation::Economic,
        )
        .map(|_| ())
        .map_err(|_| PeerError::Unauthorized)
}
pub(super) fn status(
    original: &OriginalReceipt,
    status: &ReceiptStatus,
    minimum: SourceMinima,
    revision: u64,
) -> Result<(), PeerError> {
    let o = original.original();
    if status.request != o.request_id
        || status.account != o.account_id
        || status.device != o.device_id
        || status.current.membership_revision != revision
        || !status.current.admits(minimum)
    {
        return Err(PeerError::Unauthorized);
    }
    match status.phase {
        ReceiptPhase::Unknown
            if status.operation.as_bytes() == &[0; 16] && status.binding == [0; 32] => {}
        ReceiptPhase::Pending | ReceiptPhase::Rejected { .. } | ReceiptPhase::Committed { .. }
            if status.operation == original.operation()
                && status.binding == original.binding_digest() => {}
        _ => return Err(PeerError::Unauthorized),
    }
    if let ReceiptPhase::Rejected { sequence } | ReceiptPhase::Committed { sequence } = status.phase
        && (sequence.0 == 0 || sequence > status.current.event)
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
