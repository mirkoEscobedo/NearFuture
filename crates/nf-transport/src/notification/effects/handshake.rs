use super::{NotifyEndpoint, NotifyPolicy, NotifySession};
use crate::{
    PeerError,
    auth::ServerPin,
    notification::{NotifyBody, NotifyHandshakeTranscript, NotifyRecord, PROTOCOL},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipState, PublicIdentity},
};
use std::time::{Duration, Instant};
/// Retains the exact locally originated Hello; a received ServerHello cannot replace it.
pub(crate) struct NotifyClientHandshake {
    pub(super) flow: Option<super::NotifyFlow>,
    pub(super) sent_connection: Option<ConnectionId>,
    pub(super) local: PublicIdentity,
    pub(super) local_peer: PeerId,
    pub(super) pin: ServerPin,
    pub(super) policy: NotifyPolicy,
    pub(super) initial: Option<([u8; 32], Instant)>,
    pub(super) pending: Option<NotifySession>,
}
impl NotifyClientHandshake {
    pub(crate) fn begin(
        local: PublicIdentity,
        local_peer: PeerId,
        pin: ServerPin,
        policy: NotifyPolicy,
    ) -> Result<(Self, NotifyRecord), PeerError> {
        crate::session::local_matches(&local, local_peer)?;
        policy.limits.validate()?;
        let mut flow = super::NotifyFlow::new(policy.limits)?;
        let permit = flow.reserve_outbound(214)?;
        let nonce = crate::session::fresh()?;
        let record = NotifyRecord {
            context: policy.context([0; 16]),
            body: NotifyBody::Hello {
                account: local.account,
                device: local.device,
                nonce,
                required: 1,
                optional: 0,
                offered: policy.limits,
            },
        };
        policy.admit(&record)?;
        flow.stage(&record, permit)?;
        Ok((
            Self {
                local,
                local_peer,
                pin,
                policy,
                initial: Some((nonce, Instant::now())),
                flow: Some(flow),
                sent_connection: None,
                pending: None,
            },
            record,
        ))
    }
    pub(crate) fn invalidate(&mut self) {
        self.initial = None;
        self.pending = None;
        if let Some(flow) = &mut self.flow {
            flow.invalidate();
        }
        self.flow = None;
    }
    pub(crate) fn server_hello(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<NotifyRecord, PeerError> {
        self.pending = None;
        let (nonce, created) = self.initial.take().ok_or(PeerError::Replay)?;
        if Instant::now()
            .checked_duration_since(created)
            .is_none_or(|elapsed| elapsed >= Duration::from_secs(5))
        {
            return Err(PeerError::Replay);
        }
        if self.sent_connection.is_some_and(|c| c != connection) {
            return Err(PeerError::Session);
        }
        if peer != self.pin.peer {
            return Err(PeerError::Unauthorized);
        }
        self.policy.admit(&record)?;
        let NotifyBody::ServerHello {
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
        let transcript = NotifyHandshakeTranscript {
            client_peer: self.local_peer,
            server_peer: peer,
            client_account: self.local.account,
            client_device: self.local.device,
            server_account: self.pin.account,
            server_device: self.pin.device,
            context: record.context,
            client_nonce: nonce,
            server_nonce,
            required: 1,
            optional: 0,
            server_available: available,
            selected_caps,
            offered: self.policy.limits,
            server_limits,
            selected,
        };
        let mut flow = self.flow.take().ok_or(PeerError::Replay)?;
        flow.tighten(selected)?;
        let incoming = flow.receive(&NotifyRecord {
            context: record.context,
            body: NotifyBody::ServerHello {
                nonce: server_nonce,
                available,
                selected_caps,
                server_limits,
                selected,
                proof: proof.clone(),
            },
        })?;
        let mut session = NotifySession {
            transcript,
            local: self.local.clone(),
            endpoint: NotifyEndpoint::Client,
            connection,
            revision: proof.frontier,
            minimum: self
                .policy
                .minimum_membership
                .max(self.pin.minimum_membership),
            created,
            valid: true,
            flow,
        };
        let challenge = session
            .transcript
            .challenge(1, current.revision, PROTOCOL)?;
        session.verify(&proof, challenge, current)?;
        let outgoing = session.flow.reserve_outbound(425)?;
        let proof = session.sign(
            session
                .transcript
                .challenge(2, current.revision, PROTOCOL)?,
            current,
            key,
        )?;
        let reply = NotifyRecord {
            context: session.context(),
            body: NotifyBody::ClientProof(proof),
        };
        session.flow.stage(&reply, outgoing)?;
        session.flow.complete_inbound(incoming)?;
        self.pending = Some(session);
        Ok(reply)
    }
    pub(crate) fn finished(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
    ) -> Result<NotifySession, PeerError> {
        let mut session = self.pending.take().ok_or(PeerError::Replay)?;
        session.fresh_setup()?;
        let incoming = session.flow.receive(&record)?;
        session.record(&record, peer, connection, current)?;
        let NotifyBody::Finished(proof) = record.body else {
            return Err(PeerError::Malformed);
        };
        session.verify(
            &proof,
            session
                .transcript
                .challenge(3, current.revision, PROTOCOL)?,
            current,
        )?;
        session.flow.complete_inbound(incoming)?;
        Ok(session)
    }
}
pub(crate) struct NotifyServerHandshake {
    pub(super) pending: Option<NotifySession>,
}
impl NotifyServerHandshake {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn begin(
        hello: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        local: PublicIdentity,
        local_peer: PeerId,
        policy: NotifyPolicy,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<(Self, NotifyRecord), PeerError> {
        let created = Instant::now();
        policy.admit(&hello)?;
        crate::session::local_matches(&local, local_peer)?;
        let NotifyBody::Hello {
            account,
            device,
            nonce,
            required,
            optional,
            offered,
        } = hello.body
        else {
            return Err(PeerError::Malformed);
        };
        let transcript = NotifyHandshakeTranscript {
            client_peer: peer,
            server_peer: local_peer,
            client_account: account,
            client_device: device,
            server_account: local.account,
            server_device: local.device,
            context: policy.context(crate::session::fresh()?),
            client_nonce: nonce,
            server_nonce: crate::session::fresh()?,
            required,
            optional,
            server_available: 1,
            selected_caps: 1,
            offered,
            server_limits: policy.limits,
            selected: offered.negotiate(policy.limits)?,
        };
        let mut flow = super::NotifyFlow::new(transcript.selected)?;
        let incoming = flow.receive(&NotifyRecord {
            context: hello.context,
            body: NotifyBody::Hello {
                account,
                device,
                nonce,
                required,
                optional,
                offered,
            },
        })?;
        let outgoing = flow.reserve_outbound(493)?;
        let mut session = NotifySession {
            transcript,
            local,
            endpoint: NotifyEndpoint::Server,
            connection,
            revision: current.revision,
            minimum: policy.minimum_membership,
            created,
            valid: true,
            flow,
        };
        let proof = session.sign(
            session
                .transcript
                .challenge(1, current.revision, PROTOCOL)?,
            current,
            key,
        )?;
        let t = &session.transcript;
        let reply = NotifyRecord {
            context: t.context,
            body: NotifyBody::ServerHello {
                nonce: t.server_nonce,
                available: 1,
                selected_caps: 1,
                server_limits: t.server_limits,
                selected: t.selected,
                proof,
            },
        };
        policy.admit(&reply)?;
        session.flow.stage(&reply, outgoing)?;
        session.flow.complete_inbound(incoming)?;
        Ok((
            Self {
                pending: Some(session),
            },
            reply,
        ))
    }
    pub(crate) fn invalidate(&mut self) {
        self.pending = None;
    }
    pub(crate) fn client_proof(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<(NotifySession, NotifyRecord), PeerError> {
        let mut session = self.pending.take().ok_or(PeerError::Replay)?;
        session.fresh_setup()?;
        let incoming = session.flow.receive(&record)?;
        session.record(&record, peer, connection, current)?;
        let NotifyBody::ClientProof(proof) = record.body else {
            return Err(PeerError::Malformed);
        };
        session.verify(
            &proof,
            session
                .transcript
                .challenge(2, current.revision, PROTOCOL)?,
            current,
        )?;
        let outgoing = session.flow.reserve_outbound(425)?;
        let proof = session.sign(
            session
                .transcript
                .challenge(3, current.revision, PROTOCOL)?,
            current,
            key,
        )?;
        let reply = NotifyRecord {
            context: session.context(),
            body: NotifyBody::Finished(proof),
        };
        session.flow.stage(&reply, outgoing)?;
        session.flow.complete_inbound(incoming)?;
        Ok((session, reply))
    }
}
