use super::*;
use crate::{PeerError, auth::ServerPin};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity},
    signing::device_digest,
};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptEndpoint {
    Client,
    Server,
}
/// Trusted actual owner context, never constructed from incoming role/connection claims.
pub struct ReceiptSessionBinding {
    pub endpoint: ReceiptEndpoint,
    pub local: PublicIdentity,
    pub connection: ConnectionId,
    pub server_pin: ServerPin,
    pub minimum_membership: u64,
}
/// One live handshake. Every call needs freshly loaded durable policy from its owner.
pub struct ReceiptSession {
    handshake: ReceiptHandshake,
    binding: ReceiptSessionBinding,
    revision: u64,
    next: u8,
    created: Instant,
}
impl ReceiptSession {
    pub fn new(
        handshake: ReceiptHandshake,
        binding: ReceiptSessionBinding,
        current: &MembershipState,
    ) -> Result<Self, PeerError> {
        let c = handshake.context();
        let p = binding.server_pin;
        if p.peer != c.server_peer || p.account != c.server_account || p.device != c.server_device {
            return Err(PeerError::Unauthorized);
        }
        let s = Self {
            handshake,
            binding,
            revision: current.revision,
            next: 1,
            created: Instant::now(),
        };
        s.current(current)?;
        Ok(s)
    }
    pub fn active(&self) -> bool {
        self.next == 4
    }
    pub fn invalidate(&mut self) {
        self.next = 0
    }
    pub fn handshake(&self) -> &ReceiptHandshake {
        &self.handshake
    }
    fn expected_send(&self) -> bool {
        match self.binding.endpoint {
            ReceiptEndpoint::Server => matches!(self.next, 1 | 3),
            ReceiptEndpoint::Client => self.next == 2,
        }
    }
    fn fresh(&self) -> Result<(), PeerError> {
        if self.created.elapsed() >= Duration::from_secs(5) {
            Err(PeerError::Replay)
        } else {
            Ok(())
        }
    }
    pub fn send(
        &mut self,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<ReceiptRecord, PeerError> {
        let expected = self.expected_send();
        let stage = self.next;
        self.next = 0;
        if !expected {
            return Err(PeerError::Replay);
        }
        self.fresh()?;
        self.current(current)?;
        if key.public_key() != self.binding.local.device_key {
            return Err(PeerError::Unauthorized);
        }
        let challenge = self.handshake.challenge(stage, current.revision)?;
        let local = &self.binding.local;
        let mut proof = DeviceProof {
            scope: current.scope,
            account: local.account,
            device: local.device,
            frontier: current.revision,
            peer: local.peer.clone(),
            challenge,
            signature: [0; 64],
        };
        proof.signature = key.sign(&device_digest(&proof).map_err(|_| PeerError::Unauthorized)?);
        current
            .authorize(
                &proof,
                &local.peer,
                &challenge,
                self.minimum(),
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        let c = self.handshake.context();
        let body = match stage {
            1 => ReceiptBody::ServerHello {
                nonce: c.server_nonce,
                available: 1,
                selected_caps: 1,
                server_limits: c.server_limits,
                selected: c.selected,
                proof,
            },
            2 => ReceiptBody::ClientProof(proof),
            3 => ReceiptBody::Finished(proof),
            _ => return Err(PeerError::Replay),
        };
        let record = ReceiptRecord {
            context: c.context,
            body,
        };
        encode_body(&record, c.selected)?;
        self.next = stage + 1;
        Ok(record)
    }
    pub fn receive(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        let expected = self.expected_send();
        let stage = self.next;
        self.next = 0;
        if expected || !matches!(stage, 1..=3) {
            return Err(PeerError::Replay);
        }
        self.fresh()?;
        self.check_record(&record, peer, connection, current)?;
        let c = self.handshake.context();
        let proof = match (stage, &record.body) {
            (
                1,
                ReceiptBody::ServerHello {
                    nonce,
                    available,
                    selected_caps,
                    server_limits,
                    selected,
                    proof,
                },
            ) if *nonce == c.server_nonce
                && *available == 1
                && *selected_caps == 1
                && *server_limits == c.server_limits
                && *selected == c.selected =>
            {
                proof
            }
            (2, ReceiptBody::ClientProof(proof)) | (3, ReceiptBody::Finished(proof)) => proof,
            _ => return Err(PeerError::Malformed),
        };
        let (account, device) = match self.binding.endpoint {
            ReceiptEndpoint::Client => (c.server_account, c.server_device),
            ReceiptEndpoint::Server => (c.client_account, c.client_device),
        };
        if proof.account != account || proof.device != device {
            return Err(PeerError::Unauthorized);
        }
        let challenge = self.handshake.challenge(stage, current.revision)?;
        current
            .authorize(
                proof,
                &peer.to_bytes(),
                &challenge,
                self.minimum(),
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        self.next = stage + 1;
        Ok(())
    }
    /// Checks active envelope/current policy only. Operation-specific proof validation is still required.
    pub fn admit_record(
        &mut self,
        r: &ReceiptRecord,
        peer: PeerId,
        id: ConnectionId,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        if !self.active() {
            return Err(PeerError::Replay);
        }
        let result = self.check_record(r, peer, id, current);
        if result.is_err() {
            self.invalidate()
        };
        result
    }
    fn check_record(
        &self,
        r: &ReceiptRecord,
        peer: PeerId,
        id: ConnectionId,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        self.current(current)?;
        let c = self.handshake.context();
        let expected = if self.binding.endpoint == ReceiptEndpoint::Client {
            c.server_peer
        } else {
            c.client_peer
        };
        if peer != expected || id != self.binding.connection || r.context != c.context {
            return Err(PeerError::Session);
        }
        encode_body(r, c.selected)?;
        Ok(())
    }
    fn minimum(&self) -> u64 {
        self.binding
            .minimum_membership
            .max(self.binding.server_pin.minimum_membership)
    }
    fn current(&self, s: &MembershipState) -> Result<(), PeerError> {
        nf_identity::codec::validate_state(s).map_err(|_| PeerError::Unauthorized)?;
        let c = self.handshake.context();
        if s.scope != c.context.scope
            || s.revision != self.revision
            || s.revision < self.minimum()
            || s.owner != c.server_account
        {
            return Err(PeerError::Unauthorized);
        }
        for (account, device, peer) in [
            (c.client_account, c.client_device, c.client_peer),
            (c.server_account, c.server_device, c.server_peer),
        ] {
            let a = s.accounts.get(&account).ok_or(PeerError::Unauthorized)?;
            let d = s.devices.get(&device).ok_or(PeerError::Unauthorized)?;
            if !a.roles.contains(nf_identity::model::Roles::PLAYER)
                || d.revoked
                || d.account != account
                || d.peer != peer.to_bytes()
            {
                return Err(PeerError::Unauthorized);
            }
        }
        let local = &self.binding.local;
        let (peer, account, device) = if self.binding.endpoint == ReceiptEndpoint::Client {
            (c.client_peer, c.client_account, c.client_device)
        } else {
            (c.server_peer, c.server_account, c.server_device)
        };
        let a = s
            .accounts
            .get(&local.account)
            .ok_or(PeerError::Unauthorized)?;
        let d = s
            .devices
            .get(&local.device)
            .ok_or(PeerError::Unauthorized)?;
        if local.account != account
            || local.device != device
            || local.peer != peer.to_bytes()
            || local.account_key != a.key
            || local.device_key != d.key
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(())
    }
}
