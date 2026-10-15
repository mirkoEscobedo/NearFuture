use super::{bytes::{Encoder, Reader, hash, same}, challenge::{Owned, Principal, Window},
    error::{ManagedError, Result}, proof, state::{FullState, PendingCut, PendingPolicy}};
use crate::{PrincipalDevice, Reservation};
use nf_contract::identity::{DeviceId, EntityId, OperationId, RequestId};
use nf_identity::model::{DeviceProof, ProtectedOperation};
use nf_kernel::Frontier;
use std::collections::BTreeSet;

#[derive(Clone)]
pub(super) struct PrincipalEntry {
    pub request: RequestId,
    pub window: Window,
    pub proof: DeviceProof,
}
#[derive(Clone)]
pub(super) struct Prepare {
    pub frontier: Frontier,
    pub devices: Vec<PrincipalDevice>,
    pub reservations: Vec<Reservation>,
    pub principals: Vec<PrincipalEntry>,
    pub cut: PendingCut,
}
impl Prepare {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.devices.len() > 64 || self.reservations.len() > 64 || self.principals.len() > 64
            || self.devices.len() != self.frontier.jobs().len() || self.principals.len() != self.devices.len()
        { return Err(ManagedError::Limit); }
        let mut w = Encoder::plain(1_114_112);
        w.blob(&nf_kernel::encode_frontier(&self.frontier)?, 1_048_576)?;
        w.u32(self.devices.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        let mut previous = None;
        for device in &self.devices {
            if previous.is_some_and(|p| p >= device.request) { return Err(ManagedError::Corrupt); }
            previous = Some(device.request); w.raw(device.request.as_bytes())?; w.raw(device.device.as_bytes())?;
        }
        w.u32(self.reservations.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        let mut previous = None;
        for reservation in &self.reservations {
            if previous.is_some_and(|p| p >= reservation.operation) { return Err(ManagedError::Corrupt); }
            previous = Some(reservation.operation); w.raw(reservation.operation.as_bytes())?;
            w.raw(reservation.market.as_bytes())?; w.u64(reservation.amount)?;
        }
        w.u32(self.principals.len().try_into().map_err(|_| ManagedError::Limit)?)?;
        let mut previous = None;
        for principal in &self.principals {
            if previous.is_some_and(|p| p >= principal.request)
                || principal.window.expires <= principal.window.issued
                || principal.window.expires - principal.window.issued > 20_000
            { return Err(ManagedError::Corrupt); }
            previous = Some(principal.request); w.raw(principal.request.as_bytes())?;
            w.raw(&principal.window.nonce)?; w.u64(principal.window.issued)?; w.u64(principal.window.expires)?;
            w.blob(&proof::encode(&principal.proof)?, 300)?;
        }
        self.cut.write(&mut w)?; Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::plain(bytes, 1_114_112)?;
        let frontier = nf_kernel::decode_frontier(r.blob(1_048_576)?)?;
        let n = r.u32()?; if n > 64 { return Err(ManagedError::Limit); }
        let mut devices = Vec::new();
        for _ in 0..n { devices.push(PrincipalDevice { request: RequestId::from_bytes(r.fixed()?),
            device: DeviceId::from_bytes(r.fixed()?) }); }
        let n = r.u32()?; if n > 64 { return Err(ManagedError::Limit); }
        let mut reservations = Vec::new();
        for _ in 0..n { reservations.push(Reservation { operation: OperationId::from_bytes(r.fixed()?),
            market: EntityId::from_bytes(r.fixed()?), amount: r.u64()? }); }
        let n = r.u32()?; if n > 64 { return Err(ManagedError::Limit); }
        let mut principals = Vec::new();
        for _ in 0..n { principals.push(PrincipalEntry { request: RequestId::from_bytes(r.fixed()?),
            window: Window { nonce: r.fixed()?, issued: r.u64()?, expires: r.u64()? },
            proof: proof::decode(r.blob(300)?)? }); }
        let value = Self { frontier, devices, reservations, principals, cut: PendingCut::read(&mut r)? };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
    pub fn derive(&self, current: &FullState, owner: &Owned, source: bool) -> Result<FullState> {
        self.encode()?;
        if source && self.cut.policy != PendingPolicy::Drain { return Err(ManagedError::OperationConflict); }
        let member = current.member.as_ref().ok_or(ManagedError::Unauthorized)?;
        let frontier = hash(&nf_kernel::encode_frontier(&self.frontier)?);
        let mut checked = BTreeSet::new();
        for job in self.frontier.jobs() {
            let intent = job.intent();
            let device = self.devices.iter().find(|d| d.request == intent.request).ok_or(ManagedError::Corrupt)?;
            let principal = self.principals.iter().find(|p| p.request == intent.request).ok_or(ManagedError::Corrupt)?;
            let expected = Principal { stamp: owner.stamp.clone(), sequence: owner.sequence, operation: owner.operation,
                revision: owner.revision, state: owner.state, head: owner.head, frontier, request: intent.request,
                actor: intent.actor, device: device.device, intent: nf_kernel::intent_digest(intent)?,
                times: principal.window.clone() };
            let proof = &principal.proof;
            if proof.account != intent.actor || proof.device != device.device || !checked.insert(intent.request)
                || self.cut.admitted < principal.window.issued || self.cut.admitted >= principal.window.expires
                || self.cut.admitted > owner.times.issued || owner.times.issued >= principal.window.expires
                || owner.times.issued >= self.cut.cutoff || self.cut.cutoff <= self.cut.admitted
            { return Err(ManagedError::Authentication); }
            let actual = member.devices.get(&device.device).ok_or(ManagedError::Unauthorized)?;
            member.authorize(proof, &actual.peer, &hash(&expected.transcript()?), member.revision,
                ProtectedOperation::Economic)?;
        }
        let mut next = current.clone();
        next.core = crate::transitions::prepare(&current.core, &self.frontier, &self.devices, &self.reservations)?;
        next.policy = Some(self.cut); next.encode()?; Ok(next)
    }
}
