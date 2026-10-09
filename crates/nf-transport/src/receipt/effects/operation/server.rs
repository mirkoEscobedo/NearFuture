use super::super::{ReceiptRepo, ReceiptSelector, selector::ObservedReply};
use super::common::*;
use super::delivery::{Delivery, Origin};
use crate::{PeerError, receipt::*};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_identity::model::DeviceProof;
use std::time::Instant;
struct Pending {
    operation: ReceiptOperation,
    created: Instant,
}
pub(crate) struct ReceiptOperationServer {
    pub(super) session: ReceiptSession,
    pub(super) delivery: Delivery,
    pending: Option<Pending>,
}
impl ReceiptOperationServer {
    pub(crate) fn new(session: ReceiptSession) -> Result<Self, PeerError> {
        if !session.active() {
            return Err(PeerError::Session);
        }
        Ok(Self {
            session,
            delivery: Delivery::default(),
            pending: None,
        })
    }
    pub(in crate::receipt_effects) fn awaiting_proof(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn invalidate(&mut self) {
        self.delivery.clear();
        self.pending = None;
        self.session.invalidate();
    }
    pub(crate) fn begin(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<ReceiptRecord, PeerError> {
        if self.delivery.occupied() {
            return Err(PeerError::Backpressure);
        }
        if let Some(p) = &self.pending {
            if fresh(p.created).is_ok() {
                return Err(PeerError::Backpressure);
            }
            self.delivery.clear();
            self.pending = None;
        }
        let created = Instant::now();
        repo.require_context(&record.context)?;
        repo.with_current_read(|cut| {
            fresh(created)?;
            self.session
                .admit_record(&record, peer, connection, cut.membership)?;
            let c = self.session.handshake().context();
            if cut.local.account != c.server_account || cut.local.device != c.server_device {
                return Err(PeerError::Unauthorized);
            }
            let ReceiptBody::Begin {
                target,
                nonce: client_nonce,
                minimum,
            } = record.body
            else {
                return Err(PeerError::Malformed);
            };
            let operation = ReceiptOperation {
                context_digest: self.session.handshake().context_digest()?,
                target,
                client_nonce,
                server_nonce: nonce()?,
                frontier: cut.membership.revision,
                minimum,
            };
            let response = ReceiptRecord {
                context: record.context,
                body: ReceiptBody::Challenge {
                    target,
                    client_nonce,
                    server_nonce: operation.server_nonce,
                    frontier: operation.frontier,
                    challenge: operation.challenge(1, [0; 32])?,
                },
            };
            encode_body(&response, c.selected)?;
            fresh(created)?;
            self.delivery.returned(
                &response,
                c.selected,
                Origin {
                    peer,
                    connection,
                    revision: cut.membership.revision,
                    created,
                    minimum,
                },
            )?;
            self.pending = Some(Pending { operation, created });
            Ok(response)
        })
    }
    pub(crate) fn prove(
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
        let pending = self.pending.take().ok_or(PeerError::Replay)?;
        fresh(pending.created)?;
        repo.require_context(&record.context)?;
        let t = pending.operation;
        let selector = repo.with_current_read(|cut| {
            fresh(pending.created)?;
            self.session
                .admit_record(&record, peer, connection, cut.membership)?;
            let c = self.session.handshake().context();
            let ReceiptBody::Prove {
                request,
                nonce,
                proof,
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            if *request != t.target.request
                || *nonce != t.client_nonce
                || cut.membership.revision != t.frontier
                || proof.account != c.client_account
                || proof.device != c.client_device
            {
                return Err(PeerError::Unauthorized);
            }
            verify(
                &cut,
                proof,
                &peer.to_bytes(),
                &t.challenge(1, [0; 32])?,
                t.minimum.membership_revision,
            )?;
            let p = principal(c);
            Ok(ReceiptSelector {
                account: p.account,
                device: p.device,
                scope: p.scope,
                peer: p.peer,
                target: t.target,
            })
        })?;
        let observation = repo.query_target(&selector, t.frontier, t.minimum)?;
        repo.with_current_read(|cut| {
            fresh(pending.created)?;
            self.session
                .admit_record(&record, peer, connection, cut.membership)?;
            if cut.membership.revision != t.frontier {
                return Err(PeerError::Policy);
            }
            let context = self.session.handshake().context().context;
            let placeholder = DeviceProof {
                scope: cut.membership.scope,
                account: cut.local.account,
                device: cut.local.device,
                frontier: cut.membership.revision,
                peer: cut.local.peer.clone(),
                challenge: [0; 32],
                signature: [0; 64],
            };
            let body = match observation {
                ObservedReply::Status(status) => ReceiptBody::Status {
                    status,
                    proof: placeholder,
                },
                ObservedReply::Unsupported(reason, current) => ReceiptBody::Unsupported {
                    request: t.target.request,
                    reason,
                    current,
                    proof: placeholder,
                },
            };
            let mut response = ReceiptRecord { context, body };
            let challenge = t.challenge(
                2,
                reply_prefix_digest(&response, self.session.handshake().context().selected)?,
            )?;
            let proof = sign(&cut, challenge, t.minimum.membership_revision)?;
            match &mut response.body {
                ReceiptBody::Status { proof: p, .. }
                | ReceiptBody::Unsupported { proof: p, .. } => *p = proof,
                _ => return Err(PeerError::Malformed),
            }
            encode_body(&response, self.session.handshake().context().selected)?;
            fresh(pending.created)?;
            self.delivery.returned(
                &response,
                self.session.handshake().context().selected,
                Origin {
                    peer,
                    connection,
                    revision: cut.membership.revision,
                    created: pending.created,
                    minimum: t.minimum,
                },
            )?;
            Ok(response)
        })
    }
}
