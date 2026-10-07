use super::{current, placeholder, signed};
use crate::{
    PeerError,
    auth::BulkTranscript,
    records::{PeerBody, PeerRecord},
    session::{ServerSession, fresh},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipState, ProtectedOperation},
};
use sha2::{Digest, Sha256};
use std::time::Instant;
struct Pending {
    t: BulkTranscript,
    created: Instant,
}
struct Active {
    pending: Pending,
    next: u16,
    total: u64,
    digest: Sha256,
}
/// One transfer; trusted caller must obtain fresh durable membership for every method call.
pub struct BulkServer {
    session: ServerSession,
    pending: Option<Pending>,
    active: Option<Active>,
}
impl BulkServer {
    pub fn new(session: ServerSession, state: &MembershipState) -> Result<Self, PeerError> {
        current(&session.binding()?, state)?;
        Ok(Self {
            session,
            pending: None,
            active: None,
        })
    }
    pub fn begin(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| crate::query::fresh_time(p.created, now).is_err())
        {
            self.pending = None;
        }
        if self
            .active
            .as_ref()
            .is_some_and(|a| crate::query::fresh_time(a.pending.created, now).is_err())
        {
            self.active = None;
        }
        if self.pending.is_some() || self.active.is_some() {
            return Err(PeerError::Backpressure);
        }
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.client_peer)?;
        let PeerBody::BeginBulk {
            descriptor,
            nonce,
            minimum_membership,
        } = r.body
        else {
            return Err(PeerError::Malformed);
        };
        descriptor.validate(b.context.selected)?;
        let t = BulkTranscript {
            context_digest: b.context.context_digest()?,
            descriptor,
            client_nonce: nonce,
            server_nonce: fresh()?,
            frontier: state.revision,
            minimum_membership,
        };
        let response = PeerRecord {
            context: r.context,
            body: PeerBody::BulkChallenge {
                transfer: descriptor.transfer,
                client_nonce: nonce,
                server_nonce: t.server_nonce,
                frontier: t.frontier,
                challenge: t.challenge(1, [0; 32])?,
            },
        };
        self.pending = Some(Pending { t, created: now });
        Ok(response)
    }
    pub fn prove(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        key: &SecretSeed,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let p = self.pending.take().ok_or(PeerError::Replay)?;
        crate::query::fresh_time(p.created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.client_peer)?;
        let PeerBody::ProveBulk {
            transfer,
            nonce,
            proof,
        } = r.body
        else {
            return Err(PeerError::Malformed);
        };
        if transfer != p.t.descriptor.transfer
            || nonce != p.t.client_nonce
            || state.revision != p.t.frontier
            || proof.account != b.context.client_account
            || proof.device != b.context.client_device
        {
            return Err(PeerError::Unauthorized);
        }
        state
            .authorize(
                &proof,
                &peer.to_bytes(),
                &p.t.challenge(1, [0; 32])?,
                p.t.minimum_membership,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        let response = signed(
            PeerRecord {
                context: r.context,
                body: PeerBody::BulkReady {
                    transfer,
                    proof: placeholder(&b, state),
                },
            },
            p.t,
            &b,
            state,
            key,
        )?;
        self.active = Some(Active {
            pending: p,
            next: 0,
            total: 0,
            digest: Sha256::new(),
        });
        Ok(response)
    }
    pub fn chunk(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        key: &SecretSeed,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let mut a = self.active.take().ok_or(PeerError::Replay)?;
        crate::query::fresh_time(a.pending.created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.client_peer)?;
        let PeerBody::BulkChunk {
            transfer,
            index,
            bytes,
        } = r.body
        else {
            return Err(PeerError::Malformed);
        };
        let t = a.pending.t;
        if transfer != t.descriptor.transfer || index != a.next || state.revision != t.frontier {
            return Err(PeerError::Replay);
        }
        let total = a
            .total
            .checked_add(bytes.len() as u64)
            .ok_or(PeerError::Limit)?;
        let next = a.next + 1;
        if total > t.descriptor.total || next > t.descriptor.chunks {
            return Err(PeerError::Limit);
        }
        a.digest.update(&bytes);
        a.next = next;
        a.total = total;
        let body = if next == t.descriptor.chunks {
            let digest: [u8; 32] = a.digest.clone().finalize().into();
            if total != t.descriptor.total || digest != t.descriptor.digest {
                return Err(PeerError::Malformed);
            }
            PeerBody::BulkVerified {
                transfer,
                total,
                digest,
                proof: placeholder(&b, state),
            }
        } else {
            if total >= t.descriptor.total {
                return Err(PeerError::Limit);
            }
            PeerBody::BulkProgress {
                transfer,
                next_index: next,
                accepted_total: total,
                proof: placeholder(&b, state),
            }
        };
        let response = signed(
            PeerRecord {
                context: r.context,
                body,
            },
            t,
            &b,
            state,
            key,
        )?;
        if next < t.descriptor.chunks {
            self.active = Some(a);
        }
        Ok(response)
    }
}
