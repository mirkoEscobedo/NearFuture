use super::{current, verify};
use crate::{
    PeerError,
    auth::BulkTranscript,
    records::{BulkDescriptor, PeerBody, PeerRecord},
    session::{ClientSession, fresh, sign_challenge},
};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::{keys::SecretSeed, model::MembershipState};
use sha2::{Digest, Sha256};
use std::time::Instant;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BulkResult {
    Progress {
        next_index: u16,
        accepted_total: u64,
    },
    Verified {
        transfer: [u8; 16],
        total: u64,
        digest: [u8; 32],
    },
}
struct Transfer {
    t: BulkTranscript,
    created: Instant,
    next: u16,
    total: u64,
    digest: Sha256,
}
enum Stage {
    Begin {
        descriptor: BulkDescriptor,
        nonce: [u8; 32],
        minimum: u64,
        created: Instant,
    },
    AwaitReady(Transfer),
    Active(Transfer),
    AwaitProgress(Transfer),
}
/// Explicit trusted current membership per method; results certify opaque digest bytes only.
pub struct BulkClient {
    session: ClientSession,
    stage: Option<Stage>,
}
impl BulkClient {
    pub fn new(session: ClientSession, state: &MembershipState) -> Result<Self, PeerError> {
        current(&session.binding()?, state)?;
        Ok(Self {
            session,
            stage: None,
        })
    }
    pub fn begin(
        &mut self,
        descriptor: BulkDescriptor,
        state: &MembershipState,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        if self.stage.is_some() {
            return Err(PeerError::Backpressure);
        }
        let b = self.session.binding()?;
        current(&b, state)?;
        descriptor.validate(b.context.selected)?;
        let nonce = fresh()?;
        let minimum = state.revision.max(b.minimum);
        let r = PeerRecord {
            context: b.context.context,
            body: PeerBody::BeginBulk {
                descriptor,
                nonce,
                minimum_membership: minimum,
            },
        };
        self.stage = Some(Stage::Begin {
            descriptor,
            nonce,
            minimum,
            created: now,
        });
        Ok(r)
    }
    pub fn challenge(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        key: &SecretSeed,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let Some(Stage::Begin {
            descriptor,
            nonce,
            minimum,
            created,
        }) = self.stage.take()
        else {
            return Err(PeerError::Replay);
        };
        crate::query::fresh_time(created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.server_peer)?;
        let PeerBody::BulkChallenge {
            transfer,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } = r.body
        else {
            return Err(PeerError::Malformed);
        };
        if transfer != descriptor.transfer || client_nonce != nonce || frontier != state.revision {
            return Err(PeerError::Unauthorized);
        }
        let t = BulkTranscript {
            context_digest: b.context.context_digest()?,
            descriptor,
            client_nonce,
            server_nonce,
            frontier,
            minimum_membership: minimum,
        };
        let expected = t.challenge(1, [0; 32])?;
        if challenge != expected {
            return Err(PeerError::Unauthorized);
        }
        let proof = sign_challenge(r.context, expected, state, b.local, key, minimum)?;
        self.stage = Some(Stage::AwaitReady(Transfer {
            t,
            created,
            next: 0,
            total: 0,
            digest: Sha256::new(),
        }));
        Ok(PeerRecord {
            context: r.context,
            body: PeerBody::ProveBulk {
                transfer,
                nonce,
                proof,
            },
        })
    }
    pub fn ready(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        now: Instant,
    ) -> Result<(), PeerError> {
        let Some(Stage::AwaitReady(a)) = self.stage.take() else {
            return Err(PeerError::Replay);
        };
        crate::query::fresh_time(a.created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.server_peer)?;
        let PeerBody::BulkReady { transfer, .. } = &r.body else {
            return Err(PeerError::Malformed);
        };
        if *transfer != a.t.descriptor.transfer {
            return Err(PeerError::Unauthorized);
        }
        verify(&r, a.t, &b, state)?;
        self.stage = Some(Stage::Active(a));
        Ok(())
    }
    pub fn chunk(
        &mut self,
        bytes: &[u8],
        state: &MembershipState,
        now: Instant,
    ) -> Result<PeerRecord, PeerError> {
        let Some(Stage::Active(mut a)) = self.stage.take() else {
            return Err(PeerError::Replay);
        };
        crate::query::fresh_time(a.created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        if bytes.is_empty() || bytes.len() > b.context.selected.chunk_bytes as usize {
            return Err(PeerError::Limit);
        }
        let next = a.next + 1;
        let total = a
            .total
            .checked_add(bytes.len() as u64)
            .ok_or(PeerError::Limit)?;
        if next > a.t.descriptor.chunks
            || total > a.t.descriptor.total
            || next < a.t.descriptor.chunks && total >= a.t.descriptor.total
        {
            return Err(PeerError::Limit);
        }
        a.digest.update(bytes);
        if next == a.t.descriptor.chunks
            && (total != a.t.descriptor.total
                || <[u8; 32]>::from(a.digest.clone().finalize()) != a.t.descriptor.digest)
        {
            return Err(PeerError::Malformed);
        }
        let record = PeerRecord {
            context: b.context.context,
            body: PeerBody::BulkChunk {
                transfer: a.t.descriptor.transfer,
                index: a.next,
                bytes: bytes.to_vec(),
            },
        };
        a.next = next;
        a.total = total;
        self.stage = Some(Stage::AwaitProgress(a));
        Ok(record)
    }
    pub fn progress(
        &mut self,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        now: Instant,
    ) -> Result<BulkResult, PeerError> {
        let Some(Stage::AwaitProgress(a)) = self.stage.take() else {
            return Err(PeerError::Replay);
        };
        crate::query::fresh_time(a.created, now)?;
        let b = self.session.binding()?;
        current(&b, state)?;
        b.record(&r, peer, id, b.context.server_peer)?;
        verify(&r, a.t, &b, state)?;
        let result = match r.body {
            PeerBody::BulkProgress {
                transfer,
                next_index,
                accepted_total,
                ..
            } if transfer == a.t.descriptor.transfer
                && next_index == a.next
                && accepted_total == a.total
                && a.next < a.t.descriptor.chunks =>
            {
                BulkResult::Progress {
                    next_index,
                    accepted_total,
                }
            }
            PeerBody::BulkVerified {
                transfer,
                total,
                digest,
                ..
            } if transfer == a.t.descriptor.transfer
                && total == a.total
                && total == a.t.descriptor.total
                && digest == a.t.descriptor.digest
                && a.next == a.t.descriptor.chunks =>
            {
                BulkResult::Verified {
                    transfer,
                    total,
                    digest,
                }
            }
            _ => return Err(PeerError::Replay),
        };
        if matches!(result, BulkResult::Progress { .. }) {
            self.stage = Some(Stage::Active(a));
        }
        Ok(result)
    }
}
