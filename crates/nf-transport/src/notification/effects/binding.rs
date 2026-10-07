use crate::{
    PeerError,
    notification::{NotifyHandshakeTranscript, NotifyRecord, PROTOCOL, encode_body},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity, Roles},
    signing::device_digest,
};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NotifyEndpoint {
    Client,
    Server,
}
/// Live actual connection binding, never obtainable by a record decoder.
pub(crate) struct NotifySession {
    pub(super) transcript: NotifyHandshakeTranscript,
    pub(super) local: PublicIdentity,
    pub(super) endpoint: NotifyEndpoint,
    pub(super) connection: ConnectionId,
    pub(super) revision: u64,
    pub(super) minimum: u64,
    pub(super) created: Instant,
    pub(super) valid: bool,
    pub(super) flow: super::NotifyFlow,
}
impl NotifySession {
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
        self.flow.invalidate();
    }
    pub(crate) fn active(&self) -> bool {
        self.valid
    }
    pub(crate) fn context(&self) -> crate::notification::NotifyContext {
        self.transcript.context
    }
    pub(crate) fn limits(&self) -> crate::notification::NotifyLimits {
        self.transcript.selected
    }
    pub(super) fn current(&self, s: &MembershipState) -> Result<(), PeerError> {
        nf_identity::codec::validate_state(s).map_err(|_| PeerError::Unauthorized)?;
        let t = &self.transcript;
        if !self.valid
            || s.scope != t.context.scope
            || s.revision != self.revision
            || s.revision < self.minimum
            || s.owner != t.server_account
        {
            return Err(PeerError::Unauthorized);
        }
        for (a, d, p) in [
            (t.client_account, t.client_device, t.client_peer),
            (t.server_account, t.server_device, t.server_peer),
        ] {
            let account = s.accounts.get(&a).ok_or(PeerError::Unauthorized)?;
            let device = s.devices.get(&d).ok_or(PeerError::Unauthorized)?;
            if device.revoked
                || device.account != a
                || device.peer != p.to_bytes()
                || !account.roles.contains(Roles::PLAYER)
            {
                return Err(PeerError::Unauthorized);
            }
        }
        let account = s
            .accounts
            .get(&self.local.account)
            .ok_or(PeerError::Unauthorized)?;
        let device = s
            .devices
            .get(&self.local.device)
            .ok_or(PeerError::Unauthorized)?;
        if account.key != self.local.account_key
            || device.key != self.local.device_key
            || device.peer != self.local.peer
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(())
    }
    pub(super) fn fresh_setup(&self) -> Result<(), PeerError> {
        if Instant::now()
            .checked_duration_since(self.created)
            .is_none_or(|elapsed| elapsed >= Duration::from_secs(5))
        {
            return Err(PeerError::Replay);
        }
        Ok(())
    }
    pub(super) fn remote(&self) -> PeerId {
        match self.endpoint {
            NotifyEndpoint::Client => self.transcript.server_peer,
            NotifyEndpoint::Server => self.transcript.client_peer,
        }
    }
    pub(super) fn record(
        &self,
        r: &NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        self.current(current)?;
        if peer != self.remote()
            || connection != self.connection
            || r.context != self.transcript.context
        {
            return Err(PeerError::Session);
        }
        encode_body(r, PROTOCOL, self.transcript.selected)?;
        Ok(())
    }
    pub(super) fn verify(
        &self,
        p: &DeviceProof,
        challenge: [u8; 32],
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        self.current(current)?;
        let (a, d) = match self.endpoint {
            NotifyEndpoint::Client => (
                self.transcript.server_account,
                self.transcript.server_device,
            ),
            NotifyEndpoint::Server => (
                self.transcript.client_account,
                self.transcript.client_device,
            ),
        };
        if p.account != a || p.device != d {
            return Err(PeerError::Unauthorized);
        }
        current
            .authorize(
                p,
                &self.remote().to_bytes(),
                &challenge,
                self.minimum,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        Ok(())
    }
    pub(super) fn sign(
        &self,
        challenge: [u8; 32],
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<DeviceProof, PeerError> {
        self.current(current)?;
        if key.public_key() != self.local.device_key {
            return Err(PeerError::Unauthorized);
        }
        let mut proof = DeviceProof {
            scope: current.scope,
            account: self.local.account,
            device: self.local.device,
            frontier: current.revision,
            peer: self.local.peer.clone(),
            challenge,
            signature: [0; 64],
        };
        proof.signature = key.sign(&device_digest(&proof).map_err(|_| PeerError::Unauthorized)?);
        current
            .authorize(
                &proof,
                &self.local.peer,
                &challenge,
                self.minimum,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        Ok(proof)
    }
}
