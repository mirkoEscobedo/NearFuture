use super::super::ReceiptRepo;
use super::delivery::{Delivery, Origin};
use super::{common::*, model::*};
use crate::{PeerError, receipt::*};
use libp2p::{PeerId, swarm::ConnectionId};
use std::time::Instant;
enum Pending {
    Begin {
        nonce: [u8; 32],
        minimum: SourceMinima,
        created: Instant,
    },
    Reply {
        operation: ReceiptOperation,
        created: Instant,
    },
}
pub(crate) struct ReceiptOperationClient {
    pub(super) session: ReceiptSession,
    pub(super) delivery: Delivery,
    connection: ConnectionId,
    pub(super) original: OriginalReceipt,
    minimum: SourceMinima,
    pending: Option<Pending>,
    query_not_after: Option<Instant>,
}
impl ReceiptOperationClient {
    pub(crate) fn new(
        session: ReceiptSession,
        connection: ConnectionId,
        original: OriginalReceipt,
        minimum: SourceMinima,
    ) -> Result<Self, PeerError> {
        if !session.active() {
            return Err(PeerError::Session);
        }
        let c = session.handshake().context();
        let o = original.original();
        let pin = original.source();
        if o.account_id != c.client_account
            || o.device_id != c.client_device
            || o.universe_id != c.context.scope.universe
            || o.history_id != c.context.scope.history
            || pin.peer != c.server_peer
            || pin.account != c.server_account
            || pin.device != c.server_device
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(Self {
            session,
            delivery: Delivery::default(),
            connection,
            original,
            minimum,
            pending: None,
            query_not_after: None,
        })
    }
    pub(super) fn require_query_live(&self) -> Result<(), PeerError> {
        if self
            .query_not_after
            .is_some_and(|end| Instant::now() >= end)
        {
            Err(PeerError::Replay)
        } else {
            Ok(())
        }
    }
    pub(crate) fn invalidate(&mut self) {
        self.delivery.clear();
        self.pending = None;
        self.query_not_after = None;
        self.session.invalidate();
    }
    pub(crate) fn begin(&mut self, repo: &mut ReceiptRepo) -> Result<ReceiptRecord, PeerError> {
        self.begin_inner(
            repo,
            SourceMinima {
                event: nf_contract::identity::EventSeq(0),
                store_revision: 0,
                membership_revision: 0,
            },
            None,
        )
    }
    pub(in crate::receipt_effects) fn begin_for_query(
        &mut self,
        repo: &mut ReceiptRepo,
        minimum: SourceMinima,
        not_after: Instant,
    ) -> Result<ReceiptRecord, PeerError> {
        self.begin_inner(repo, minimum, Some(not_after))
    }
    fn begin_inner(
        &mut self,
        repo: &mut ReceiptRepo,
        query_minimum: SourceMinima,
        not_after: Option<Instant>,
    ) -> Result<ReceiptRecord, PeerError> {
        if self.pending.is_some() {
            return Err(PeerError::Backpressure);
        }
        self.query_not_after = not_after;
        self.require_query_live()?;
        repo.validate_original(&self.original)?;
        self.require_query_live()?;
        repo.require_context(&self.session.handshake().context().context)?;
        let retained = repo.protected_minimum(&self.original)?;
        self.require_query_live()?;
        let nonce = nonce()?;
        let created = Instant::now();
        let (record, minimum) = repo.with_current_read(|cut| {
            fresh(created)?;
            self.require_query_live()?;
            let c = self.session.handshake().context();
            if cut.local.account != c.client_account || cut.local.device != c.client_device {
                return Err(PeerError::Unauthorized);
            }
            let minimum = SourceMinima {
                membership_revision: self
                    .minimum
                    .membership_revision
                    .max(retained.membership_revision)
                    .max(cut.membership.revision)
                    .max(self.original.source().minimum_membership)
                    .max(query_minimum.membership_revision),
                event: self
                    .minimum
                    .event
                    .max(retained.event)
                    .max(query_minimum.event),
                store_revision: self
                    .minimum
                    .store_revision
                    .max(retained.store_revision)
                    .max(query_minimum.store_revision),
            };
            let record = ReceiptRecord {
                context: c.context,
                body: ReceiptBody::Begin {
                    target: target(&self.original),
                    nonce,
                    minimum,
                },
            };
            let peer = c.server_peer;
            let selected = c.selected;
            self.session
                .admit_record(&record, peer, self.connection, cut.membership)?;
            self.delivery.returned(
                &record,
                selected,
                Origin {
                    peer,
                    connection: self.connection,
                    revision: cut.membership.revision,
                    created,
                    minimum,
                },
            )?;
            Ok((record, minimum))
        })?;
        self.pending = Some(Pending::Begin {
            nonce,
            minimum,
            created,
        });
        Ok(record)
    }
    pub(crate) fn challenge(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<ReceiptRecord, PeerError> {
        if self.delivery.queued() {
            self.invalidate();
            return Err(PeerError::Replay);
        }
        self.delivery.clear_prepared()?;
        let Some(Pending::Begin {
            nonce,
            minimum,
            created,
        }) = self.pending.take()
        else {
            return Err(PeerError::Replay);
        };
        fresh(created)?;
        self.require_query_live()?;
        repo.require_context(&record.context)?;
        repo.with_current_read(|cut| {
            fresh(created)?;
            self.require_query_live()?;
            self.session
                .admit_record(&record, peer, connection, cut.membership)?;
            let ReceiptBody::Challenge {
                target: t,
                client_nonce,
                server_nonce,
                frontier,
                challenge,
            } = record.body
            else {
                return Err(PeerError::Malformed);
            };
            if t != target(&self.original)
                || client_nonce != nonce
                || frontier != cut.membership.revision
            {
                return Err(PeerError::Unauthorized);
            }
            let operation = ReceiptOperation {
                context_digest: self.session.handshake().context_digest()?,
                target: t,
                client_nonce,
                server_nonce,
                frontier,
                minimum,
            };
            let expected = operation.challenge(1, [0; 32])?;
            if challenge != expected {
                return Err(PeerError::Unauthorized);
            }
            let proof = sign(&cut, expected, minimum.membership_revision)?;
            let response = ReceiptRecord {
                context: record.context,
                body: ReceiptBody::Prove {
                    request: t.request,
                    nonce,
                    proof,
                },
            };
            encode_body(&response, self.session.handshake().context().selected)?;
            self.delivery.returned(
                &response,
                self.session.handshake().context().selected,
                Origin {
                    peer,
                    connection,
                    revision: cut.membership.revision,
                    created,
                    minimum,
                },
            )?;
            self.pending = Some(Pending::Reply { operation, created });
            Ok(response)
        })
    }
    pub(crate) fn reply(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<VerifiedReceipt, PeerError> {
        if self.delivery.queued() {
            self.invalidate();
            return Err(PeerError::Replay);
        }
        self.delivery.clear_prepared()?;
        let Some(Pending::Reply { operation, created }) = self.pending.take() else {
            return Err(PeerError::Replay);
        };
        fresh(created)?;
        self.require_query_live()?;
        repo.require_context(&record.context)?;
        repo.with_current_read(|cut| {
            fresh(created)?;
            self.require_query_live()?;
            self.session
                .admit_record(&record, peer, connection, cut.membership)?;
            let c = self.session.handshake().context();
            if cut.membership.revision != operation.frontier {
                return Err(PeerError::Policy);
            }
            let (proof, result) = match &record.body {
                ReceiptBody::Status { status: s, proof } => {
                    status(
                        &self.original,
                        s,
                        operation.minimum,
                        cut.membership.revision,
                    )?;
                    (proof, VerifiedResult::Status(s.clone()))
                }
                ReceiptBody::Unsupported {
                    request,
                    reason,
                    current,
                    proof,
                } => {
                    if *request != self.original.request()
                        || current.membership_revision != cut.membership.revision
                        || (*reason == ReceiptUnsupportedReason::BindingConflict
                            && !current.admits(operation.minimum))
                    {
                        return Err(PeerError::Unauthorized);
                    }
                    (proof, VerifiedResult::Unsupported(*reason))
                }
                _ => return Err(PeerError::Malformed),
            };
            if proof.account != c.server_account || proof.device != c.server_device {
                return Err(PeerError::Unauthorized);
            }
            let challenge = operation.challenge(2, reply_prefix_digest(&record, c.selected)?)?;
            verify(
                &cut,
                proof,
                &peer.to_bytes(),
                &challenge,
                operation.minimum.membership_revision,
            )?;
            Ok(VerifiedReceipt {
                context: c.context,
                original: self.original.clone(),
                result,
                minimum: operation.minimum,
                revision: cut.membership.revision,
                created,
                not_after: self.query_not_after,
            })
        })
    }
}
