use super::{NotifyLimits, NotifySelector};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, OperationId, RequestId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};
pub(super) struct Reader<'a> {
    pub input: &'a [u8],
    pub position: usize,
}
impl Reader<'_> {
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], PeerError> {
        let end = self.position.checked_add(N).ok_or(PeerError::Limit)?;
        let value = self
            .input
            .get(self.position..end)
            .ok_or(PeerError::Malformed)?
            .try_into()
            .map_err(|_| PeerError::Malformed)?;
        self.position = end;
        Ok(value)
    }
    pub fn u8(&mut self) -> Result<u8, PeerError> {
        Ok(self.array::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, PeerError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, PeerError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, PeerError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
}
pub(super) fn read_limits(r: &mut Reader<'_>) -> Result<NotifyLimits, PeerError> {
    let value = NotifyLimits {
        frame: r.u16()?,
        queue_bytes: r.u32()?,
        queue_items: r.u16()?,
        rate: r.u16()?,
        burst: r.u16()?,
        pending: r.u16()?,
    };
    value.validate()?;
    Ok(value)
}
pub(super) fn write_limits(b: &mut Vec<u8>, value: NotifyLimits) {
    b.extend(value.frame.to_le_bytes());
    b.extend(value.queue_bytes.to_le_bytes());
    for n in [value.queue_items, value.rate, value.burst, value.pending] {
        b.extend(n.to_le_bytes());
    }
}
pub(super) fn read_selector(r: &mut Reader<'_>) -> Result<NotifySelector, PeerError> {
    Ok(NotifySelector {
        request: RequestId::from_bytes(r.array()?),
        operation: OperationId::from_bytes(r.array()?),
        binding: r.array()?,
    })
}
pub(super) fn write_selector(b: &mut Vec<u8>, value: &NotifySelector) {
    b.extend(value.request.as_bytes());
    b.extend(value.operation.as_bytes());
    b.extend(value.binding);
}
pub(super) fn read_proof(r: &mut Reader<'_>, scope: Scope) -> Result<DeviceProof, PeerError> {
    let proof_scope = Scope {
        universe: UniverseId::from_bytes(r.array()?),
        history: HistoryId::from_bytes(r.array()?),
    };
    let account = AccountId::from_bytes(r.array()?);
    let device = DeviceId::from_bytes(r.array()?);
    let frontier = r.u64()?;
    let length = r.u8()? as usize;
    if length == 0 || length > 128 {
        return Err(PeerError::Limit);
    }
    let field = r.array::<128>()?;
    if field[length..].iter().any(|v| *v != 0) {
        return Err(PeerError::Malformed);
    }
    let peer = libp2p::PeerId::from_bytes(&field[..length]).map_err(|_| PeerError::Malformed)?;
    if peer.to_bytes() != field[..length] {
        return Err(PeerError::Malformed);
    }
    let proof = DeviceProof {
        scope: proof_scope,
        account,
        device,
        frontier,
        peer: field[..length].to_vec(),
        challenge: r.array()?,
        signature: r.array()?,
    };
    if proof.scope != scope {
        return Err(PeerError::Scope);
    }
    Ok(proof)
}
