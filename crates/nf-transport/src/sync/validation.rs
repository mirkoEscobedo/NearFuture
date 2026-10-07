use super::*;
use crate::PeerError;
use nf_identity::model::{DeviceProof, Scope};
fn id(v: &[u8; 16]) -> Result<(), PeerError> {
    if *v == [0; 16] {
        Err(PeerError::Malformed)
    } else {
        Ok(())
    }
}
fn pins(v: ExpectedProfilePins, p: SyncWirePolicy) -> Result<(), PeerError> {
    if v == p.pins {
        Ok(())
    } else {
        Err(PeerError::Unsupported)
    }
}
pub(super) fn proof(v: &DeviceProof, scope: Scope) -> Result<(), PeerError> {
    if v.scope != scope {
        return Err(PeerError::Scope);
    }
    id(v.account.as_bytes())?;
    id(v.device.as_bytes())?;
    if !(1..=128).contains(&v.peer.len()) {
        return Err(PeerError::Limit);
    }
    let peer = libp2p::PeerId::from_bytes(&v.peer).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != v.peer {
        return Err(PeerError::Malformed);
    }
    Ok(())
}
pub(super) fn lane(kind: u8, lane: SyncLane) -> Result<(), PeerError> {
    if !(1..=17).contains(&kind) {
        return Err(PeerError::Unsupported);
    }
    if (matches!(kind,5..=9|16..=17) && lane != SyncLane::Control)
        || (matches!(kind, 10..=15) && lane != SyncLane::Transfer)
    {
        return Err(PeerError::Scope);
    }
    Ok(())
}
pub(super) fn validate(r: &SyncRecord, p: SyncWirePolicy) -> Result<usize, PeerError> {
    p.validate()?;
    if r.lane != p.lane {
        return Err(PeerError::Scope);
    }
    let kind = r.body.kind();
    lane(kind, r.lane)?;
    id(r.context.scope.universe.as_bytes())?;
    id(r.context.scope.history.as_bytes())?;
    if (kind == 1) != (r.context.session == [0; 16]) {
        return Err(PeerError::Session);
    }
    let n = HEADER_BYTES
        .checked_add(r.body.bytes()?)
        .ok_or(PeerError::Limit)?;
    if n > p.frame_maximum()? {
        return Err(PeerError::Limit);
    }
    let s = r.context.scope;
    match &r.body {
        SyncBody::Hello(v) => {
            id(v.account.as_bytes())?;
            id(v.device.as_bytes())?;
            v.offered.validate()?;
            pins(v.pins, p)?;
            if v.required != 1 || v.optional != 0 {
                return Err(PeerError::Unsupported);
            }
        }
        SyncBody::ServerHello(v) => {
            v.offered.validate()?;
            v.selected.validate()?;
            if v.available != 1 || v.selected_caps != 1 {
                return Err(PeerError::Unsupported);
            }
            if v.selected.negotiate(v.offered)? != v.selected
                || v.selected.negotiate(p.limits)? != v.selected
            {
                return Err(PeerError::Limit);
            }
            proof(&v.proof, s)?;
        }
        SyncBody::ClientProof(v) | SyncBody::Finished(v) => proof(v, s)?,
        SyncBody::BeginSync(v) => {
            pins(v.pins, p)?;
            id(&v.request.0)?;
            if matches!(v.mode, SyncMode::Delta) != v.base.is_some() {
                return Err(PeerError::Malformed);
            }
            if !(1..=257).contains(&v.maximum_documents)
                || !(1..=8388608).contains(&v.maximum_data_bytes)
            {
                return Err(PeerError::Limit);
            }
        }
        SyncBody::SyncChallenge(v) => id(&v.request.0)?,
        SyncBody::ProveSync(v) => {
            id(&v.request.0)?;
            proof(&v.proof, s)?;
        }
        SyncBody::ManifestOffer(v) => {
            id(&v.request.0)?;
            id(&v.export.0)?;
            if !(1..=65536).contains(&v.manifest_length)
                || !(1..=257).contains(&v.document_count)
                || !(1..=8388608).contains(&v.data_bytes)
            {
                return Err(PeerError::Limit);
            }
            proof(&v.proof, s)?;
        }
        SyncBody::GapRequiresSnapshot(v) => {
            id(&v.request.0)?;
            proof(&v.proof, s)?;
        }
        SyncBody::BeginDocument(v) => {
            id(&v.transfer.0)?;
            id(&v.export.0)?;
            p.limits.document(v.total, v.count)?;
        }
        SyncBody::DocumentChallenge(v) => {
            id(&v.transfer.0)?;
            id(&v.export.0)?;
        }
        SyncBody::ProveDocument(v) => {
            id(&v.transfer.0)?;
            proof(&v.proof, s)?;
        }
        SyncBody::DocumentReady(v) => {
            id(&v.transfer.0)?;
            id(&v.export.0)?;
            p.limits.document(v.total, v.count)?;
            proof(&v.proof, s)?;
        }
        SyncBody::SyncChunk(v) => {
            id(&v.transfer.0)?;
            id(&v.export.0)?;
            p.limits.chunk(v.total, v.count, v.index, v.data.len())?;
        }
        SyncBody::DocumentEnd(v) => {
            id(&v.transfer.0)?;
            id(&v.export.0)?;
            if v.received == 0 || v.received > p.limits.document_maximum()? {
                return Err(PeerError::Limit);
            }
            proof(&v.proof, s)?;
        }
        SyncBody::ReplicaInstallReceipt(v) => {
            id(&v.export.0)?;
            if v.generation == 0 {
                return Err(PeerError::Malformed);
            }
            proof(&v.proof, s)?;
        }
        SyncBody::SyncRefused(v) => {
            id(&v.request.0)?;
            if let Some(e) = v.export {
                id(&e.0)?;
            }
            proof(&v.proof, s)?;
        }
    }
    Ok(n)
}
