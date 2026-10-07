use super::{QueryResult, current, fresh_time};
use crate::{
    PeerError,
    auth::QueryTranscript,
    records::{Lane, PeerBody, PeerRecord, reply_prefix_digest},
    session::{ClientSession, fresh, sign_challenge},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_contract::identity::RequestId;
use nf_identity::{keys::SecretSeed, model::ProtectedOperation};
use nf_store::Store;
use std::time::Instant;
enum Pending {
    Begin {
        request: RequestId,
        nonce: [u8; 32],
        created: Instant,
    },
    Reply {
        transcript: QueryTranscript,
        created: Instant,
    },
}
/// Trusted local repository is actual SQLite; every begin/proof/reply reloads current durable membership.
pub struct QueryClient {
    session: ClientSession,
    store: Store,
    pending: Option<Pending>,
}
impl QueryClient {
    pub fn new(session: ClientSession, mut store: Store) -> Result<Self, PeerError> {
        let b = session.binding()?;
        if b.context.lane != Lane::Control || b.context.selected_caps & 1 == 0 {
            return Err(PeerError::Unsupported);
        }
        current(&mut store, &b)?;
        Ok(Self {
            session,
            store,
            pending: None,
        })
    }
    /// Trusted background owner composition only, never a decoded network operation.
    pub fn owner_store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    pub fn begin(&mut self, request: RequestId, now: Instant) -> Result<PeerRecord, PeerError> {
        if self.pending.is_some() {
            return Err(PeerError::Backpressure);
        }
        let b = self.session.binding()?;
        let state = current(&mut self.store, &b)?;
        let nonce = fresh()?;
        let record = PeerRecord {
            context: b.context.context,
            body: PeerBody::BeginQuery {
                request,
                nonce,
                minimum_membership: state.revision.max(b.minimum),
            },
        };
        self.pending = Some(Pending::Begin {
            request,
            nonce,
            created: now,
        });
        Ok(record)
    }
    pub fn challenge(
        &mut self,
        record: PeerRecord,
        peer: PeerId,
        connection: ConnectionId,
        key: &SecretSeed,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let Some(Pending::Begin {
            request,
            nonce,
            created,
        }) = self.pending.take()
        else {
            return Err(PeerError::Replay);
        };
        fresh_time(created, now)?;
        let b = self.session.binding()?;
        let state = current(&mut self.store, &b)?;
        b.record(&record, peer, connection, b.context.server_peer)?;
        let PeerBody::QueryChallenge {
            request: r,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        if r != request || client_nonce != nonce || frontier != state.revision {
            return Err(PeerError::Unauthorized);
        }
        let transcript = QueryTranscript {
            context_digest: b.context.context_digest()?,
            request,
            client_nonce,
            server_nonce,
            frontier,
            minimum_membership: state.revision.max(b.minimum),
        };
        let expected = transcript.challenge(1, [0; 32])?;
        if challenge != expected {
            return Err(PeerError::Unauthorized);
        }
        let proof = sign_challenge(
            record.context,
            expected,
            &state,
            b.local,
            key,
            transcript.minimum_membership,
        )?;
        let response = PeerRecord {
            context: record.context,
            body: PeerBody::ProveQuery {
                request,
                nonce,
                proof,
            },
        };
        self.pending = Some(Pending::Reply {
            transcript,
            created,
        });
        Ok(response)
    }
    pub fn reply(
        &mut self,
        record: PeerRecord,
        peer: PeerId,
        connection: ConnectionId,
        now: Instant,
    ) -> Result<QueryResult, PeerError> {
        let Some(Pending::Reply {
            transcript,
            created,
        }) = self.pending.take()
        else {
            return Err(PeerError::Replay);
        };
        fresh_time(created, now)?;
        let b = self.session.binding()?;
        let state = current(&mut self.store, &b)?;
        b.record(&record, peer, connection, b.context.server_peer)?;
        let (request, proof, result) = match &record.body {
            PeerBody::RetainedStatus {
                request,
                phase,
                proof,
            } => (*request, proof, QueryResult::Status(phase.clone())),
            PeerBody::Unsupported {
                request,
                reason,
                proof,
            } => (*request, proof, QueryResult::Unsupported(*reason)),
            _ => return Err(PeerError::Malformed),
        };
        if request != transcript.request
            || state.revision != transcript.frontier
            || proof.account != b.context.server_account
            || proof.device != b.context.server_device
        {
            return Err(PeerError::Unauthorized);
        }
        let challenge = transcript.challenge(
            2,
            reply_prefix_digest(&record, Lane::Control, b.context.selected)?,
        )?;
        state
            .authorize(
                proof,
                &peer.to_bytes(),
                &challenge,
                transcript.minimum_membership,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        Ok(result)
    }
}
