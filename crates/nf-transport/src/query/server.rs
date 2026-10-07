use super::{current, fresh_time};
use crate::{
    PeerError,
    auth::QueryTranscript,
    records::{Lane, PeerBody, PeerRecord, RetainedPhase, UnsupportedReason, reply_prefix_digest},
    session::{ServerSession, fresh, sign_challenge},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, ProtectedOperation},
};
use nf_store::{RequestStatus, Store};
use std::time::Instant;
struct Pending {
    transcript: QueryTranscript,
    created: Instant,
}
/// One outstanding operation per connection; this is below all advertised item ceilings.
pub struct QueryServer {
    session: ServerSession,
    store: Store,
    pending: Option<Pending>,
}
impl QueryServer {
    pub fn new(session: ServerSession, mut store: Store) -> Result<Self, PeerError> {
        let binding = session.binding()?;
        if binding.context.lane != Lane::Control || binding.context.selected_caps & 1 == 0 {
            return Err(PeerError::Unsupported);
        }
        current(&mut store, &binding)?;
        Ok(Self {
            session,
            store,
            pending: None,
        })
    }
    /// Trusted background owner composition only; never reachable from a transport record.
    pub fn owner_store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    pub fn begin(
        &mut self,
        record: PeerRecord,
        peer: PeerId,
        connection: ConnectionId,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        if let Some(p) = &self.pending {
            if fresh_time(p.created, now).is_ok() {
                return Err(PeerError::Backpressure);
            }
            self.pending = None;
        }
        let b = self.session.binding()?;
        let state = current(&mut self.store, &b)?;
        b.record(&record, peer, connection, b.context.client_peer)?;
        let PeerBody::BeginQuery {
            request,
            nonce,
            minimum_membership,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        let transcript = QueryTranscript {
            context_digest: b.context.context_digest()?,
            request,
            client_nonce: nonce,
            server_nonce: fresh()?,
            frontier: state.revision,
            minimum_membership,
        };
        let challenge = transcript.challenge(1, [0; 32])?;
        let response = PeerRecord {
            context: b.context.context,
            body: PeerBody::QueryChallenge {
                request,
                client_nonce: nonce,
                server_nonce: transcript.server_nonce,
                frontier: state.revision,
                challenge,
            },
        };
        self.pending = Some(Pending {
            transcript,
            created: now,
        });
        Ok(response)
    }
    pub fn prove(
        &mut self,
        record: PeerRecord,
        peer: PeerId,
        connection: ConnectionId,
        key: &SecretSeed,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let pending = self.pending.take().ok_or(PeerError::Replay)?;
        fresh_time(pending.created, now)?;
        let b = self.session.binding()?;
        let state = current(&mut self.store, &b)?;
        b.record(&record, peer, connection, b.context.client_peer)?;
        let PeerBody::ProveQuery {
            request,
            nonce,
            proof,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        let t = pending.transcript;
        if request != t.request
            || nonce != t.client_nonce
            || state.revision != t.frontier
            || proof.account != b.context.client_account
            || proof.device != b.context.client_device
        {
            return Err(PeerError::Unauthorized);
        }
        let challenge = t.challenge(1, [0; 32])?;
        state
            .authorize(
                &proof,
                &peer.to_bytes(),
                &challenge,
                t.minimum_membership,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        let retained = self
            .store
            .query_bound(request, proof.account, proof.device, state.scope)
            .map_err(|_| PeerError::Unauthorized)?;
        let placeholder = DeviceProof {
            scope: state.scope,
            account: b.local.account,
            device: b.local.device,
            frontier: state.revision,
            peer: b.local.peer.clone(),
            challenge: [0; 32],
            signature: [0; 64],
        };
        let body = match retained {
            None => PeerBody::RetainedStatus {
                request,
                phase: RetainedPhase::UnknownRequest,
                proof: placeholder,
            },
            Some(r) => match r.status {
                RequestStatus::Pending { operation } => PeerBody::RetainedStatus {
                    request,
                    phase: RetainedPhase::Pending {
                        operation,
                        binding: r.binding_digest,
                    },
                    proof: placeholder,
                },
                RequestStatus::Committed {
                    operation,
                    sequence,
                    rejection: Some(_),
                } => PeerBody::RetainedStatus {
                    request,
                    phase: RetainedPhase::Rejected {
                        operation,
                        binding: r.binding_digest,
                        sequence,
                    },
                    proof: placeholder,
                },
                RequestStatus::Committed {
                    rejection: None, ..
                } => PeerBody::Unsupported {
                    request,
                    reason: UnsupportedReason::KernelOutcome,
                    proof: placeholder,
                },
            },
        };
        let mut response = PeerRecord {
            context: b.context.context,
            body,
        };
        let challenge = t.challenge(
            2,
            reply_prefix_digest(&response, Lane::Control, b.context.selected)?,
        )?;
        let proof = sign_challenge(
            response.context,
            challenge,
            &state,
            b.local,
            key,
            t.minimum_membership,
        )?;
        match &mut response.body {
            PeerBody::RetainedStatus { proof: p, .. } | PeerBody::Unsupported { proof: p, .. } => {
                *p = proof
            }
            _ => return Err(PeerError::Malformed),
        };
        Ok(response)
    }
    pub(crate) fn into_store(mut self) -> Store {
        self.session.invalidate();
        self.pending = None;
        self.store
    }
}
