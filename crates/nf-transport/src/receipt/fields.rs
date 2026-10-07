use super::{RECEIPT_PROFILE, ReceiptTarget, SourceMinima, reader::Reader};
use crate::{PeerError, records::PeerLimits};
use nf_contract::identity::*;
use nf_identity::model::{DeviceProof, Scope};
pub(super) fn read_limits(r: &mut Reader<'_>) -> Result<PeerLimits, PeerError> {
    let l = PeerLimits {
        control_frame: r.u32()?,
        bulk_frame: r.u32()?,
        chunk_bytes: r.u32()?,
        control_queue_bytes: r.u32()?,
        bulk_queue_bytes: r.u32()?,
        control_items: r.u16()?,
        bulk_items: r.u16()?,
        pending_challenges: r.u16()?,
    };
    l.validate()?;
    Ok(l)
}
pub(super) fn profile(r: &mut Reader<'_>) -> Result<(), PeerError> {
    if r.u16()? == RECEIPT_PROFILE {
        Ok(())
    } else {
        Err(PeerError::Unsupported)
    }
}
pub(super) fn nonzero<const N: usize>(v: [u8; N]) -> Result<[u8; N], PeerError> {
    if v == [0; N] {
        Err(PeerError::Malformed)
    } else {
        Ok(v)
    }
}
pub(super) fn target(r: &mut Reader<'_>) -> Result<ReceiptTarget, PeerError> {
    let target = ReceiptTarget {
        request: RequestId::from_bytes(r.array()?),
        operation: OperationId::from_bytes(r.array()?),
        binding: r.array()?,
    };
    target.validate()?;
    profile(r)?;
    Ok(target)
}
pub(super) fn current(r: &mut Reader<'_>) -> Result<SourceMinima, PeerError> {
    Ok(SourceMinima {
        store_revision: r.u64()?,
        event: EventSeq(r.u64()?),
        membership_revision: r.u64()?,
    })
}
pub(super) fn read_proof(r: &mut Reader<'_>, scope: Scope) -> Result<DeviceProof, PeerError> {
    let proof_scope = Scope {
        universe: UniverseId::from_bytes(r.array()?),
        history: HistoryId::from_bytes(r.array()?),
    };
    if proof_scope != scope {
        return Err(PeerError::Scope);
    }
    let account = AccountId::from_bytes(nonzero(r.array()?)?);
    let device = DeviceId::from_bytes(nonzero(r.array()?)?);
    let frontier = r.u64()?;
    let n = r.u8()? as usize;
    if !(1..=128).contains(&n) {
        return Err(PeerError::Limit);
    }
    let field = r.array::<128>()?;
    if field[n..].iter().any(|b| *b != 0) {
        return Err(PeerError::Malformed);
    }
    let peer = libp2p::PeerId::from_bytes(&field[..n]).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != field[..n] {
        return Err(PeerError::Malformed);
    }
    Ok(DeviceProof {
        scope,
        account,
        device,
        frontier,
        peer: field[..n].to_vec(),
        challenge: r.array()?,
        signature: r.array()?,
    })
}
