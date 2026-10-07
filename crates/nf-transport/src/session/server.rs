use super::{SessionPolicy, fresh, local_matches, sign_stage};
use crate::{
    PeerError,
    auth::HandshakeContext,
    records::{Lane, PeerBody, PeerContext, PeerRecord},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipState, ProtectedOperation, PublicIdentity},
};
/// One connection-owned state; current membership must come from the trusted durable owner on each call.
pub struct ServerSession {
    local: PublicIdentity,
    context: HandshakeContext,
    connection: ConnectionId,
    policy: SessionPolicy,
    revision: u64,
    next: u8,
}
impl ServerSession {
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        hello: PeerRecord,
        actual_client: PeerId,
        connection: ConnectionId,
        local: PublicIdentity,
        local_peer: PeerId,
        lane: Lane,
        policy: SessionPolicy,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<(Self, PeerRecord), PeerError> {
        policy.admit(&hello, lane)?;
        local_matches(&local, local_peer)?;
        if current.owner != local.account {
            return Err(PeerError::Unauthorized);
        }
        let PeerBody::Hello {
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
        let context = HandshakeContext {
            lane,
            client_peer: actual_client,
            server_peer: local_peer,
            client_account: account,
            client_device: device,
            server_account: local.account,
            server_device: local.device,
            context: policy.context(fresh()?),
            client_nonce: nonce,
            server_nonce: fresh()?,
            required,
            optional,
            server_available: 3,
            selected_caps: (required | optional) & 3,
            offered,
            server_limits: policy.limits,
            selected: offered.negotiate(policy.limits)?,
        };
        context.validate()?;
        let proof = sign_stage(&context, 1, current, &local, key, policy.minimum_membership)?;
        let reply = PeerRecord {
            context: context.context,
            body: PeerBody::ServerHello {
                nonce: context.server_nonce,
                available: 3,
                selected_caps: context.selected_caps,
                server_limits: context.server_limits,
                selected: context.selected,
                proof,
            },
        };
        Ok((
            Self {
                local,
                context,
                connection,
                policy,
                revision: current.revision,
                next: 2,
            },
            reply,
        ))
    }
    pub fn active(&self) -> bool {
        self.next == 4
    }
    pub fn invalidate(&mut self) {
        self.next = 0;
    }
    pub fn client_proof(
        &mut self,
        record: PeerRecord,
        actual_peer: PeerId,
        connection: ConnectionId,
        current: &MembershipState,
        key: &SecretSeed,
    ) -> Result<PeerRecord, PeerError> {
        let expected = self.next;
        self.next = 0;
        if expected != 2 {
            return Err(PeerError::Replay);
        }
        if actual_peer != self.context.client_peer
            || connection != self.connection
            || record.context != self.context.context
        {
            return Err(PeerError::Session);
        }
        self.policy.admit(&record, self.context.lane)?;
        if current.scope != self.policy.scope
            || current.revision != self.revision
            || current.owner != self.local.account
        {
            return Err(PeerError::Unauthorized);
        }
        let PeerBody::ClientProof(proof) = record.body else {
            return Err(PeerError::Malformed);
        };
        if proof.account != self.context.client_account
            || proof.device != self.context.client_device
        {
            return Err(PeerError::Unauthorized);
        }
        let challenge = self.context.challenge(2, current.revision)?;
        current
            .authorize(
                &proof,
                &actual_peer.to_bytes(),
                &challenge,
                self.policy.minimum_membership,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        let proof = sign_stage(
            &self.context,
            3,
            current,
            &self.local,
            key,
            self.policy.minimum_membership,
        )?;
        self.next = 4;
        Ok(PeerRecord {
            context: self.context.context,
            body: PeerBody::Finished(proof),
        })
    }
    pub fn context(&self) -> Option<PeerContext> {
        self.active().then_some(self.context.context)
    }
    pub(crate) fn binding(&self) -> Result<super::Binding<'_>, PeerError> {
        if !self.active() {
            return Err(PeerError::Session);
        }
        Ok(super::Binding {
            context: &self.context,
            local: &self.local,
            connection: self.connection,
            revision: self.revision,
            minimum: self.policy.minimum_membership,
        })
    }
    pub(crate) fn limits(&self) -> crate::records::PeerLimits {
        self.context.selected
    }
}
