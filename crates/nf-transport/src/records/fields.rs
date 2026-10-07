use super::{PeerLimits, reader::Reader};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
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
pub(crate) fn write_limits(b: &mut Vec<u8>, l: PeerLimits) {
    for v in [
        l.control_frame,
        l.bulk_frame,
        l.chunk_bytes,
        l.control_queue_bytes,
        l.bulk_queue_bytes,
    ] {
        b.extend(v.to_le_bytes());
    }
    for v in [l.control_items, l.bulk_items, l.pending_challenges] {
        b.extend(v.to_le_bytes());
    }
}
pub(super) fn read_peer(r: &mut Reader<'_>) -> Result<Vec<u8>, PeerError> {
    let n = r.u8()? as usize;
    if n == 0 || n > 128 {
        return Err(PeerError::Limit);
    }
    let field = r.array::<128>()?;
    if field[n..].iter().any(|v| *v != 0) {
        return Err(PeerError::Malformed);
    }
    let peer = libp2p::PeerId::from_bytes(&field[..n]).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != field[..n] {
        return Err(PeerError::Malformed);
    }
    Ok(field[..n].to_vec())
}
pub(crate) fn write_peer(b: &mut Vec<u8>, peer: &[u8]) -> Result<(), PeerError> {
    if peer.is_empty() || peer.len() > 128 {
        return Err(PeerError::Limit);
    }
    let parsed = libp2p::PeerId::from_bytes(peer).map_err(|_| PeerError::Malformed)?;
    if parsed.to_bytes() != peer {
        return Err(PeerError::Malformed);
    }
    b.push(peer.len() as u8);
    b.extend(peer);
    b.resize(b.len() + 128 - peer.len(), 0);
    Ok(())
}
pub(super) fn read_proof(r: &mut Reader<'_>, scope: Scope) -> Result<DeviceProof, PeerError> {
    let proof = DeviceProof {
        scope: Scope {
            universe: UniverseId::from_bytes(r.array()?),
            history: HistoryId::from_bytes(r.array()?),
        },
        account: AccountId::from_bytes(r.array()?),
        device: DeviceId::from_bytes(r.array()?),
        frontier: r.u64()?,
        peer: read_peer(r)?,
        challenge: r.array()?,
        signature: r.array()?,
    };
    if proof.scope != scope {
        return Err(PeerError::Scope);
    }
    Ok(proof)
}
pub(crate) fn write_proof(b: &mut Vec<u8>, p: &DeviceProof) -> Result<(), PeerError> {
    b.extend(p.scope.universe.as_bytes());
    b.extend(p.scope.history.as_bytes());
    b.extend(p.account.as_bytes());
    b.extend(p.device.as_bytes());
    b.extend(p.frontier.to_le_bytes());
    write_peer(b, &p.peer)?;
    b.extend(p.challenge);
    b.extend(p.signature);
    Ok(())
}
