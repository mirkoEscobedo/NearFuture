use super::{NotifyEndpoint, NotifySession};
use crate::{
    PeerError,
    notification::{NotifyBody, NotifyRecord, NotifySelector, NotifySubscribeTranscript, PROTOCOL},
    receipt::OriginalReceipt,
    receipt_effects::ReceiptRepo,
};
use libp2p::{PeerId, swarm::ConnectionId};
use std::time::{Duration, Instant};
struct Pending {
    original: OriginalReceipt,
    subscription: [u8; 16],
    nonce: [u8; 32],
    lifetime: u16,
    created: Instant,
    transcript: Option<NotifySubscribeTranscript>,
}
struct Active {
    original: OriginalReceipt,
    transcript: NotifySubscribeTranscript,
    created: Instant,
    next_sequence: u64,
}
fn live(p: &Pending) -> Result<(), PeerError> {
    let elapsed = Instant::now()
        .checked_duration_since(p.created)
        .ok_or(PeerError::Replay)?;
    if elapsed >= Duration::from_secs(5) || elapsed >= Duration::from_secs(u64::from(p.lifetime)) {
        return Err(PeerError::Replay);
    }
    Ok(())
}
/// Client-owned Begin time and immutable original; no notice may replace either.
pub(crate) struct NotifySubscriber {
    pub(super) session: NotifySession,
    pending: Option<Pending>,
    active: Option<Active>,
    original: Option<OriginalReceipt>,
    generation: u64,
    covered: u64,
}
impl NotifySubscriber {
    pub(crate) fn new(session: NotifySession) -> Result<Self, PeerError> {
        if session.endpoint != NotifyEndpoint::Client || !session.active() {
            return Err(PeerError::Session);
        }
        session.fresh_setup()?;
        Ok(Self {
            session,
            pending: None,
            active: None,
            original: None,
            generation: 0,
            covered: 0,
        })
    }
    pub(crate) fn invalidate(&mut self) {
        self.pending = None;
        self.active = None;
        self.session.invalidate();
    }
    pub(crate) fn begin_subscription(
        &mut self,
        original: OriginalReceipt,
        lifetime: u16,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        let created = Instant::now();
        self.session.fresh_setup()?;
        if self.pending.is_some() || self.active.is_some() {
            return Err(PeerError::Replay);
        }
        self.session.with_repo(repo, |_| Ok(()))?;
        let selector = repo.selector(&original)?.target();
        let subscription = crate::session::fresh()?;
        let nonce = crate::session::fresh()?;
        let record = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::BeginSubscribe {
                subscription,
                selector: NotifySelector {
                    request: selector.request,
                    operation: selector.operation,
                    binding: selector.binding,
                },
                nonce,
                minimum_membership: self.session.minimum,
                lifetime,
            },
        };
        super::elapsed_cut(created, Duration::from_secs(5_u64.min(u64::from(lifetime))))?;
        crate::notification::encode_body(&record, PROTOCOL, self.session.limits())?;
        let permit = self.session.flow.reserve_outbound(251)?;
        self.session.flow.stage(&record, permit)?;
        self.original = Some(original.clone());
        self.pending = Some(Pending {
            original,
            subscription,
            nonce,
            lifetime,
            created,
            transcript: None,
        });
        Ok(record)
    }
    pub(crate) fn challenge(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        let mut pending = self.pending.take().ok_or(PeerError::Replay)?;
        let result = self.accept_challenge(record, peer, connection, repo, &mut pending);
        match result {
            Ok(reply) => {
                self.pending = Some(pending);
                Ok(reply)
            }
            Err(error) => {
                self.invalidate();
                Err(error)
            }
        }
    }
    fn accept_challenge(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        pending: &mut Pending,
    ) -> Result<NotifyRecord, PeerError> {
        live(pending)?;
        if pending.transcript.is_some() {
            return Err(PeerError::Replay);
        }
        let inbound = self.session.flow.receive(&record)?;
        self.session.with_repo(repo, |cut| {
            live(pending)?;
            self.session
                .record(&record, peer, connection, cut.membership)
        })?;
        let NotifyBody::SubscribeChallenge {
            subscription,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        if subscription != pending.subscription
            || client_nonce != pending.nonce
            || frontier != self.session.revision
        {
            return Err(PeerError::Replay);
        }
        let original = &pending.original;
        let transcript = NotifySubscribeTranscript {
            context_digest: self.session.transcript.context_digest(PROTOCOL)?,
            subscription,
            selector: NotifySelector {
                request: original.request(),
                operation: original.operation(),
                binding: original.binding_digest(),
            },
            client_nonce,
            server_nonce,
            frontier,
            minimum_membership: self.session.minimum,
            lifetime: pending.lifetime,
        };
        if transcript.challenge(1, [0; 32])? != challenge {
            return Err(PeerError::Unauthorized);
        }
        let permit = self.session.flow.reserve_outbound(473)?;
        let proof = self.session.with_repo(repo, |cut| {
            live(pending)?;
            self.session.sign(challenge, cut.membership, cut.device_key)
        })?;
        live(pending)?;
        pending.transcript = Some(transcript);
        let reply = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::ProveSubscribe {
                subscription,
                nonce: server_nonce,
                proof,
            },
        };
        self.session.flow.stage(&reply, permit)?;
        self.session.flow.complete_inbound(inbound)?;
        Ok(reply)
    }
    pub(crate) fn subscribed(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let pending = self.pending.take().ok_or(PeerError::Replay)?;
        let result = self.accept_subscribed(record, peer, connection, repo, pending);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn accept_subscribed(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        pending: Pending,
    ) -> Result<(), PeerError> {
        live(&pending)?;
        let transcript = pending.transcript.clone().ok_or(PeerError::Replay)?;
        let inbound = self.session.flow.receive(&record)?;
        self.session.with_repo(repo, |cut| {
            live(&pending)?;
            self.session
                .record(&record, peer, connection, cut.membership)?;
            let NotifyBody::Subscribed {
                subscription,
                selector,
                first_sequence,
                lifetime,
                proof,
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            if *subscription != transcript.subscription
                || *selector != transcript.selector
                || *first_sequence != 1
                || *lifetime != transcript.lifetime
            {
                return Err(PeerError::Replay);
            }
            let prefix = crate::notification::signed_prefix_digest(
                &record,
                PROTOCOL,
                self.session.limits(),
            )?;
            self.session
                .verify(proof, transcript.challenge(2, prefix)?, cut.membership)
        })?;
        live(&pending)?;
        self.session.flow.complete_inbound(inbound)?;
        self.generation = self.generation.checked_add(1).ok_or(PeerError::Limit)?;
        self.active = Some(Active {
            original: pending.original,
            transcript,
            created: pending.created,
            next_sequence: 1,
        });
        Ok(())
    }
}

impl NotifySubscriber {
    pub(in crate::notification_effects) fn subscription_completion_end(
        &self,
    ) -> Result<Instant, PeerError> {
        let p = self.pending.as_ref().ok_or(PeerError::Replay)?;
        live(p)?;
        p.created
            .checked_add(Duration::from_secs(5_u64.min(u64::from(p.lifetime))))
            .ok_or(PeerError::Replay)
    }
    pub(in crate::notification_effects) fn live_output(&self) -> Result<(), PeerError> {
        if let Some(p) = &self.pending {
            live(p)
        } else {
            self.active_live()
        }
    }
    pub(super) fn followup_generation(&self) -> Result<u64, PeerError> {
        if self.dirty() {
            Ok(self.generation)
        } else {
            Err(PeerError::Backpressure)
        }
    }
    pub(super) fn cover_generation(&mut self, generation: u64) -> Result<(), PeerError> {
        if generation == 0 || generation > self.generation || generation <= self.covered {
            return Err(PeerError::Replay);
        }
        self.covered = generation;
        Ok(())
    }
}

#[path = "client/notice.rs"]
mod notice;
