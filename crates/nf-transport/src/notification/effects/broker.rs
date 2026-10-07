use super::{NotifyEndpoint, NotifySession};
use crate::{
    PeerError,
    notification::{NotifyBody, NotifyRecord, NotifySubscribeTranscript, PROTOCOL, encode_body},
    receipt::ReceiptTarget,
    receipt_effects::{ProjectionRead, ReceiptRepo, ReceiptSelector},
};
use libp2p::{PeerId, swarm::ConnectionId};
use std::time::{Duration, Instant};
struct PendingSubscription {
    transcript: NotifySubscribeTranscript,
    selector: ReceiptSelector,
    baseline: ProjectionRead,
    created: Instant,
}
fn pending_live(p: &PendingSubscription) -> Result<(), PeerError> {
    super::elapsed_cut(
        p.created,
        Duration::from_secs(5_u64.min(u64::from(p.transcript.lifetime))),
    )
}
struct ActiveSubscription {
    transcript: NotifySubscribeTranscript,
    selector: ReceiptSelector,
    baseline: crate::receipt::ReceiptPhase,
    created: Instant,
    dirty: bool,
    flushed: bool,
    next_sequence: u64,
    awaited: Option<Awaited>,
}
fn active_live(a: &ActiveSubscription) -> Result<(), PeerError> {
    super::elapsed_cut(
        a.created,
        Duration::from_secs(u64::from(a.transcript.lifetime)),
    )?;
    if let Some(n) = &a.awaited {
        super::elapsed_cut(n.created, Duration::from_secs(5))?;
    }
    Ok(())
}
struct Awaited {
    transcript: crate::notification::NotifyNoticeTranscript,
    prefix: [u8; 32],
    created: Instant,
}
/// Borrows the one repository for every protected action. Never owns or opens a Store.
pub(crate) struct NotifyBroker {
    pub(super) session: NotifySession,
    pending: Option<PendingSubscription>,
    active: Option<ActiveSubscription>,
    // Original setup/Ack origin survives consumption until exact output custody finishes.
    completion_end: Option<Instant>,
}
impl NotifyBroker {
    pub(crate) fn new(session: NotifySession) -> Result<Self, PeerError> {
        if session.endpoint != NotifyEndpoint::Server || !session.active() {
            return Err(PeerError::Session);
        }
        session.fresh_setup()?;
        Ok(Self {
            session,
            pending: None,
            active: None,
            completion_end: None,
        })
    }
    #[cfg(test)]
    pub(in crate::notification_effects) fn pending_setup_end_for_test(
        &self,
    ) -> Result<Instant, PeerError> {
        self.pending
            .as_ref()
            .ok_or(PeerError::Replay)?
            .created
            .checked_add(Duration::from_secs(5))
            .ok_or(PeerError::Replay)
    }
    pub(crate) fn invalidate(&mut self) {
        self.pending = None;
        self.active = None;
        self.completion_end = None;
        self.session.invalidate();
    }
    pub(crate) fn begin_subscription(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        let result = self.begin(record, peer, connection, repo);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn begin(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        self.session.fresh_setup()?;
        if self.pending.is_some() || self.active.is_some() {
            return Err(PeerError::Replay);
        }
        let inbound = self.session.flow.receive(&record)?;
        let remote = self.session.remote_from_repo(peer, connection, repo)?;
        if record.context != self.session.context() {
            return Err(PeerError::Session);
        }
        encode_body(&record, PROTOCOL, self.session.limits())?;
        let NotifyBody::BeginSubscribe {
            subscription,
            selector,
            nonce,
            minimum_membership,
            lifetime,
        } = record.body
        else {
            return Err(PeerError::Malformed);
        };
        if minimum_membership > self.session.revision {
            return Err(PeerError::Unauthorized);
        }
        let target = ReceiptTarget {
            request: selector.request,
            operation: selector.operation,
            binding: selector.binding,
        };
        let (retained, baseline) =
            repo.admit_remote_selector(remote, target, self.session.revision)?;
        let mut server_nonce = [0; 32];
        getrandom::fill(&mut server_nonce).map_err(|_| PeerError::Storage)?;
        let transcript = NotifySubscribeTranscript {
            context_digest: self.session.transcript.context_digest(PROTOCOL)?,
            subscription,
            selector,
            client_nonce: nonce,
            server_nonce,
            frontier: self.session.revision,
            minimum_membership,
            lifetime,
        };
        let challenge = transcript.challenge(1, [0; 32])?;
        let reply = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::SubscribeChallenge {
                subscription,
                client_nonce: nonce,
                server_nonce,
                frontier: self.session.revision,
                challenge,
            },
        };
        encode_body(&reply, PROTOCOL, self.session.limits())?;
        self.session.fresh_setup()?;
        let permit = self.session.flow.reserve_outbound(248)?;
        self.session.flow.stage(&reply, permit)?;
        self.session.flow.complete_inbound(inbound)?;
        self.pending = Some(PendingSubscription {
            transcript,
            selector: retained,
            baseline,
            created: Instant::now(),
        });
        Ok(reply)
    }
    pub(crate) fn prove_subscription(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        // First attempt consumes the exact private challenge even when subsequent validation fails.
        let pending = self.pending.take().ok_or(PeerError::Replay)?;
        let result = self.prove(record, peer, connection, repo, pending);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn prove(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        pending: PendingSubscription,
    ) -> Result<NotifyRecord, PeerError> {
        let elapsed = Instant::now()
            .checked_duration_since(pending.created)
            .ok_or(PeerError::Replay)?;
        if elapsed >= Duration::from_secs(5)
            || elapsed >= Duration::from_secs(u64::from(pending.transcript.lifetime))
        {
            return Err(PeerError::Replay);
        }
        let inbound = self.session.flow.receive(&record)?;
        self.session.with_repo(repo, |cut| {
            pending_live(&pending)?;
            self.session
                .record(&record, peer, connection, cut.membership)?;
            let NotifyBody::ProveSubscribe {
                subscription,
                nonce,
                proof,
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            if *subscription != pending.transcript.subscription
                || *nonce != pending.transcript.server_nonce
            {
                return Err(PeerError::Replay);
            }
            self.session.verify(
                proof,
                pending.transcript.challenge(1, [0; 32])?,
                cut.membership,
            )
        })?;
        let projection = repo.observe_selector(&pending.selector, self.session.revision)?;
        let mut reply = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::Subscribed {
                subscription: pending.transcript.subscription,
                selector: pending.transcript.selector,
                first_sequence: 1,
                lifetime: pending.transcript.lifetime,
                proof: nf_identity::model::DeviceProof {
                    scope: self.session.context().scope,
                    account: self.session.local.account,
                    device: self.session.local.device,
                    frontier: self.session.revision,
                    peer: self.session.local.peer.clone(),
                    challenge: [0; 32],
                    signature: [0; 64],
                },
            },
        };
        let permit = self.session.flow.reserve_outbound(516)?;
        let prefix =
            crate::notification::signed_prefix_digest(&reply, PROTOCOL, self.session.limits())?;
        let challenge = pending.transcript.challenge(2, prefix)?;
        let (proof, admitted) = self.session.with_repo(repo, |cut| {
            pending_live(&pending)?;
            let admitted = Instant::now();
            let proof = self
                .session
                .sign(challenge, cut.membership, cut.device_key)?;
            Ok((proof, admitted))
        })?;
        if let NotifyBody::Subscribed { proof: slot, .. } = &mut reply.body {
            *slot = proof;
        }
        pending_live(&pending)?;
        self.session.flow.stage(&reply, permit)?;
        self.session.flow.complete_inbound(inbound)?;
        self.completion_end = Some(
            pending
                .created
                .checked_add(Duration::from_secs(5))
                .ok_or(PeerError::Replay)?,
        );
        self.active = Some(ActiveSubscription {
            transcript: pending.transcript,
            selector: pending.selector,
            baseline: projection.phase,
            created: admitted,
            dirty: pending.baseline.phase != projection.phase,
            flushed: false,
            next_sequence: 1,
            awaited: None,
        });
        Ok(reply)
    }
}

#[path = "broker/control.rs"]
mod control;
#[path = "broker/notice.rs"]
mod notice;
