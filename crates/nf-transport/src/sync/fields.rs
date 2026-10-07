use super::{
    reader::{Reader, Writer},
    *,
};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, EventSeq, HistoryId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};
pub(super) fn nonzero<const N: usize>(value: [u8; N]) -> Result<[u8; N], PeerError> {
    if value == [0; N] {
        Err(PeerError::Malformed)
    } else {
        Ok(value)
    }
}
pub(super) fn limits(r: &mut Reader<'_>) -> Result<SyncLimits, PeerError> {
    let v = SyncLimits {
        control_frame: r.u32()?,
        transfer_frame: r.u32()?,
        chunk_bytes: r.u32()?,
        control_queue_bytes: r.u32()?,
        transfer_queue_bytes: r.u32()?,
        control_items: r.u16()?,
        transfer_items: r.u16()?,
        pending_challenges: r.u16()?,
    };
    v.validate()?;
    Ok(v)
}
pub(super) fn put_limits(w: &mut Writer, v: SyncLimits) {
    for n in [
        v.control_frame,
        v.transfer_frame,
        v.chunk_bytes,
        v.control_queue_bytes,
        v.transfer_queue_bytes,
    ] {
        w.u32(n);
    }
    for n in [v.control_items, v.transfer_items, v.pending_challenges] {
        w.u16(n);
    }
}
pub(super) fn pins(r: &mut Reader<'_>) -> Result<ExpectedProfilePins, PeerError> {
    if r.u16()? != 1 {
        return Err(PeerError::Unsupported);
    }
    Ok(ExpectedProfilePins {
        implementation: r.array()?,
        schema: r.array()?,
    })
}
pub(super) fn put_pins(w: &mut Writer, v: ExpectedProfilePins) {
    w.u16(1);
    w.raw(&v.implementation);
    w.raw(&v.schema);
}
pub(super) fn point(r: &mut Reader<'_>) -> Result<Point, PeerError> {
    Ok(Point {
        store_revision: r.u64()?,
        event: EventSeq(r.u64()?),
        world: r.array()?,
        state: r.array()?,
        journal: r.array()?,
    })
}
pub(super) fn put_point(w: &mut Writer, v: Point) {
    w.u64(v.store_revision);
    w.u64(v.event.0);
    w.raw(&v.world);
    w.raw(&v.state);
    w.raw(&v.journal);
}
pub(super) fn optional_point(r: &mut Reader<'_>) -> Result<Option<Point>, PeerError> {
    match r.u8()? {
        1 => Ok(Some(point(r)?)),
        0 => {
            if r.take(112)?.iter().any(|n| *n != 0) {
                return Err(PeerError::Malformed);
            }
            Ok(None)
        }
        _ => Err(PeerError::Malformed),
    }
}
pub(super) fn put_optional_point(w: &mut Writer, v: Option<Point>) {
    match v {
        Some(v) => {
            w.u8(1);
            put_point(w, v)
        }
        None => {
            w.u8(0);
            w.raw(&[0; 112]);
        }
    }
}
pub(super) fn stamp(r: &mut Reader<'_>) -> Result<MemberStamp, PeerError> {
    Ok(MemberStamp {
        revision: r.u64()?,
        digest: r.array()?,
    })
}
pub(super) fn put_stamp(w: &mut Writer, v: MemberStamp) {
    w.u64(v.revision);
    w.raw(&v.digest);
}
pub(super) fn peer(r: &mut Reader<'_>) -> Result<libp2p::PeerId, PeerError> {
    let n = usize::from(r.u8()?);
    if !(1..=128).contains(&n) {
        return Err(PeerError::Limit);
    }
    let raw = r.array::<128>()?;
    if raw[n..].iter().any(|v| *v != 0) {
        return Err(PeerError::Malformed);
    }
    let p = libp2p::PeerId::from_bytes(&raw[..n]).map_err(|_| PeerError::Malformed)?;
    if p.to_bytes() != raw[..n] {
        return Err(PeerError::Malformed);
    }
    Ok(p)
}
pub(super) fn put_peer(w: &mut Writer, p: libp2p::PeerId) {
    let raw = p.to_bytes();
    w.u8(raw.len() as u8);
    w.raw(&raw);
    w.raw(&[0; 128][..128 - raw.len()]);
}
pub(super) fn proof(r: &mut Reader<'_>, scope: Scope) -> Result<DeviceProof, PeerError> {
    let got = Scope {
        universe: UniverseId::from_bytes(r.array()?),
        history: HistoryId::from_bytes(r.array()?),
    };
    if got != scope {
        return Err(PeerError::Scope);
    }
    let account = AccountId::from_bytes(nonzero(r.array()?)?);
    let device = DeviceId::from_bytes(nonzero(r.array()?)?);
    let frontier = r.u64()?;
    let peer = peer(r)?.to_bytes();
    Ok(DeviceProof {
        scope,
        account,
        device,
        frontier,
        peer,
        challenge: r.array()?,
        signature: r.array()?,
    })
}
pub(super) fn put_proof(w: &mut Writer, p: &DeviceProof) {
    w.raw(p.scope.universe.as_bytes());
    w.raw(p.scope.history.as_bytes());
    w.raw(p.account.as_bytes());
    w.raw(p.device.as_bytes());
    w.u64(p.frontier);
    let raw = &p.peer;
    w.u8(raw.len() as u8);
    w.raw(raw);
    w.raw(&[0; 128][..128 - raw.len()]);
    w.raw(&p.challenge);
    w.raw(&p.signature);
}
