use super::{SessionPolicy, fresh, local_matches, sign_stage};
use crate::{
    PeerError,
    auth::{ClientHandshake, HandshakeContext, ServerPin},
    records::{Lane, PeerBody, PeerContext, PeerLimits, PeerRecord},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipState, PublicIdentity},
};
/// Trusted owner supplies actual Noise event peer/connection and freshly loaded durable state on each call.
pub struct ClientSession {
    local: PublicIdentity,
    local_peer: PeerId,
    pin: ServerPin,
    lane: Lane,
    policy: SessionPolicy,
    initial: Option<([u8; 32], u32, u32, PeerLimits)>,
    pending: Option<(HandshakeContext, ClientHandshake, ConnectionId)>,
}
impl ClientSession {
    pub fn begin(
        local: PublicIdentity,
        local_peer: PeerId,
        pin: ServerPin,
        lane: Lane,
        policy: SessionPolicy,
    ) -> Result<(Self, PeerRecord), PeerError> {
        local_matches(&local, local_peer)?;
        policy.limits.validate()?;
        let nonce = fresh()?;
        let required = lane as u32;
        let optional = 3;
        let offered = policy.limits;
        let hello = PeerRecord {
            context: policy.context([0; 16]),
            body: PeerBody::Hello {
                account: local.account,
                device: local.device,
                nonce,
                required,
                optional,
                offered,
            },
        };
        Ok((
            Self {
                local,
                local_peer,
                pin,
                lane,
                policy,
                initial: Some((nonce, required, optional, offered)),
                pending: None,
            },
            hello,
        ))
    }
    pub fn active(&self) -> bool {
        self.pending.as_ref().is_some_and(|(_, v, _)| v.active())
    }
    pub fn invalidate(&mut self) {
        self.initial = None;
        self.pending = None;
    }
    pub fn server_hello(
        &mut self,
        record: PeerRecord,
        actual_peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<PeerRecord, PeerError> {
        let (nonce, required, optional, offered) = self.initial.take().ok_or(PeerError::Replay)?;
        self.pending = None;
        if actual_peer != self.pin.peer {
            return Err(PeerError::Unauthorized);
        }
        self.policy.admit(&record, self.lane)?;
        let PeerBody::ServerHello {
            nonce: server_nonce,
            available,
            selected_caps,
            server_limits,
            selected,
            proof,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        let context = HandshakeContext {
            lane: self.lane,
            client_peer: self.local_peer,
            server_peer: actual_peer,
            client_account: self.local.account,
            client_device: self.local.device,
            server_account: self.pin.account,
            server_device: self.pin.device,
            context: record.context,
            client_nonce: nonce,
            server_nonce,
            required,
            optional,
            server_available: available,
            selected_caps,
            offered,
            server_limits,
            selected,
        };
        let mut verifier = ClientHandshake::new(context.clone(), self.pin)?;
        verifier.accept_server_hello(&proof, current)?;
        let client_proof = sign_stage(
            &context,
            2,
            current,
            &self.local,
            key,
            self.policy
                .minimum_membership
                .max(self.pin.minimum_membership),
        )?;
        let reply = PeerRecord {
            context: context.context,
            body: PeerBody::ClientProof(client_proof),
        };
        self.pending = Some((context, verifier, connection));
        Ok(reply)
    }
    pub fn finished(
        &mut self,
        record: PeerRecord,
        actual_peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        let (context, mut verifier, expected) = self.pending.take().ok_or(PeerError::Replay)?;
        if actual_peer != self.pin.peer
            || connection != expected
            || record.context != context.context
        {
            return Err(PeerError::Session);
        }
        self.policy.admit(&record, self.lane)?;
        let PeerBody::Finished(proof) = record.body else {
            return Err(PeerError::Malformed);
        };
        verifier.accept_finished(&proof, current)?;
        self.pending = Some((context, verifier, expected));
        Ok(())
    }
    pub fn context(&self) -> Option<PeerContext> {
        self.pending
            .as_ref()
            .filter(|(_, v, _)| v.active())
            .map(|(c, _, _)| c.context)
    }
    pub(crate) fn binding(&self) -> Result<super::Binding<'_>, PeerError> {
        let (c, v, connection) = self
            .pending
            .as_ref()
            .filter(|(_, v, _)| v.active())
            .ok_or(PeerError::Session)?;
        Ok(super::Binding {
            context: c,
            local: &self.local,
            connection: *connection,
            revision: v.revision().ok_or(PeerError::Session)?,
            minimum: self
                .policy
                .minimum_membership
                .max(self.pin.minimum_membership),
        })
    }
}
