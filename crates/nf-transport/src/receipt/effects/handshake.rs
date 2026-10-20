use super::{ReceiptRepo, model::role, operation::fresh};
use super::{activation::Activation, operation::Delivery};
use crate::{
    PeerError,
    auth::HandshakeContext,
    receipt::*,
    records::{Lane, PeerContext, PeerLimits},
};
use libp2p::{PeerId, swarm::ConnectionId};
use std::time::Instant;
fn random<const N: usize>() -> Result<[u8; N], PeerError> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| PeerError::Entropy)?;
    if bytes == [0; N] {
        Err(PeerError::Entropy)
    } else {
        Ok(bytes)
    }
}
pub(super) enum ClientPending {
    Hello {
        nonce: [u8; 32],
        created: Instant,
    },
    Finished {
        session: Box<ReceiptSession>,
        created: Instant,
        revision: u64,
    },
}
pub(crate) struct ReceiptClientHandshake {
    pub(super) pending: Option<ClientPending>,
    pub(super) returned: Option<(ReceiptRecord, Instant, u64)>,
    pub(super) delivery: Delivery,
    pub(super) bound: Option<(PeerId, ConnectionId)>,
}
impl Default for ReceiptClientHandshake {
    fn default() -> Self {
        Self::new()
    }
}
impl ReceiptClientHandshake {
    pub(crate) fn new() -> Self {
        Self {
            pending: None,
            returned: None,
            delivery: Delivery::default(),
            bound: None,
        }
    }
    pub(crate) fn invalidate(&mut self) {
        self.bound = None;
        self.delivery.clear();
        self.returned = None;
        self.pending = None;
    }
    pub(crate) fn hello(&mut self, repo: &mut ReceiptRepo) -> Result<ReceiptRecord, PeerError> {
        if self.pending.is_some() {
            return Err(PeerError::Backpressure);
        }
        let nonce = random()?;
        let config = repo.config();
        let created = Instant::now();
        let record = repo.with_current_read(|cut| {
            fresh(created)?;
            Ok(ReceiptRecord {
                context: PeerContext {
                    session: [0; 16],
                    scope: config.scope,
                    ruleset: config.ruleset,
                    content: config.content,
                },
                body: ReceiptBody::Hello {
                    account: cut.local.account,
                    device: cut.local.device,
                    nonce,
                    required: 1,
                    optional: 0,
                    offered: PeerLimits::default(),
                },
            })
        })?;
        let revision = repo.with_current_read(|cut| {
            fresh(created)?;
            Ok(cut.membership.revision)
        })?;
        self.returned = Some((record.clone(), created, revision));
        self.pending = Some(ClientPending::Hello { nonce, created });
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
        self.returned = None;
        let Some(ClientPending::Hello { nonce, created }) = self.pending.take() else {
            return Err(PeerError::Replay);
        };
        fresh(created)?;
        repo.require_context(&record.context)?;
        let config = repo.config();
        repo.with_current_read(|cut| {
            fresh(created)?;
            if peer != config.server_pin.peer || self.bound.is_some_and(|b| b != (peer, connection))
            {
                return Err(PeerError::Unauthorized);
            }
            let ReceiptBody::ServerHello {
                nonce: server_nonce,
                available,
                selected_caps,
                server_limits,
                selected,
                ..
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            let context = HandshakeContext {
                lane: Lane::Control,
                client_peer: PeerId::from_bytes(&cut.local.peer)
                    .map_err(|_| PeerError::Unauthorized)?,
                server_peer: peer,
                client_account: cut.local.account,
                client_device: cut.local.device,
                server_account: config.server_pin.account,
                server_device: config.server_pin.device,
                context: record.context,
                client_nonce: nonce,
                server_nonce: *server_nonce,
                required: 1,
                optional: 0,
                server_available: *available,
                selected_caps: *selected_caps,
                offered: PeerLimits::default(),
                server_limits: *server_limits,
                selected: *selected,
            };
            let mut session = ReceiptSession::new(
                ReceiptHandshake::new(context)?,
                ReceiptSessionBinding {
                    endpoint: ReceiptEndpoint::Client,
                    local: cut.local.clone(),
                    connection,
                    server_pin: config.server_pin,
                    minimum_membership: config.minimum_membership,
                },
                cut.membership,
            )?;
            session.receive(record, peer, connection, cut.membership)?;
            let proof = session.send(cut.membership, cut.device_key)?;
            self.bound = Some((peer, connection));
            self.returned = Some((proof.clone(), created, cut.membership.revision));
            self.pending = Some(ClientPending::Finished {
                session: Box::new(session),
                created,
                revision: cut.membership.revision,
            });
            Ok(proof)
        })
    }
    pub(in crate::receipt_effects) fn finished(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<Activation, PeerError> {
        if self.delivery.queued() {
            self.invalidate();
            return Err(PeerError::Replay);
        }
        self.delivery.clear_prepared()?;
        self.returned = None;
        let Some(ClientPending::Finished {
            mut session,
            created,
            revision,
        }) = self.pending.take()
        else {
            return Err(PeerError::Replay);
        };
        fresh(created)?;
        repo.require_context(&record.context)?;
        repo.with_current_read(|cut| {
            fresh(created)?;
            session.receive(record, peer, connection, cut.membership)?;
            Ok(Activation::new(*session, created, revision))
        })
    }
}
pub(super) struct ServerPending {
    pub(super) session: ReceiptSession,
    pub(super) created: Instant,
}
pub(crate) struct ReceiptServerHandshake {
    pub(super) pending: Option<ServerPending>,
    pub(super) completed: Option<ReceiptSession>,
    pub(super) finished_digest: Option<[u8; 32]>,
    pub(super) returned: Option<(ReceiptRecord, Instant, u64)>,
    pub(super) delivery: Delivery,
    pub(super) bound: Option<(PeerId, ConnectionId)>,
}
impl Default for ReceiptServerHandshake {
    fn default() -> Self {
        Self::new()
    }
}
impl ReceiptServerHandshake {
    pub(crate) fn new() -> Self {
        Self {
            pending: None,
            returned: None,
            delivery: Delivery::default(),
            bound: None,
            completed: None,
            finished_digest: None,
        }
    }
    pub(crate) fn invalidate(&mut self) {
        self.finished_digest = None;
        self.completed = None;
        self.bound = None;
        self.delivery.clear();
        self.returned = None;
        self.pending = None;
    }
    pub(crate) fn begin(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<ReceiptRecord, PeerError> {
        if self.returned.is_some() || self.delivery.occupied() || self.completed.is_some() {
            return Err(PeerError::Backpressure);
        }
        if let Some(p) = &self.pending {
            if fresh(p.created).is_ok() {
                return Err(PeerError::Backpressure);
            }
            self.bound = None;
            self.delivery.clear();
            self.returned = None;
            self.pending = None;
        }
        let created = Instant::now();
        repo.require_context(&record.context)?;
        let config = repo.config();
        repo.with_current_read(|cut| {
            fresh(created)?;
            if cut.local.account != config.server_pin.account
                || cut.local.device != config.server_pin.device
            {
                return Err(PeerError::Unauthorized);
            }
            let ReceiptBody::Hello {
                account,
                device,
                nonce,
                required,
                optional,
                offered,
            } = record.body
            else {
                return Err(PeerError::Malformed);
            };
            if record.context.session != [0; 16] || required != 1 {
                return Err(PeerError::Unsupported);
            }
            role(cut.membership, account, device, peer)?;
            let server_limits = PeerLimits::default();
            let selected = offered.negotiate(server_limits)?;
            let context = HandshakeContext {
                lane: Lane::Control,
                client_peer: peer,
                server_peer: PeerId::from_bytes(&cut.local.peer)
                    .map_err(|_| PeerError::Unauthorized)?,
                client_account: account,
                client_device: device,
                server_account: cut.local.account,
                server_device: cut.local.device,
                context: PeerContext {
                    session: random()?,
                    ..record.context
                },
                client_nonce: nonce,
                server_nonce: random()?,
                required,
                optional,
                server_available: 1,
                selected_caps: 1,
                offered,
                server_limits,
                selected,
            };
            let mut session = ReceiptSession::new(
                ReceiptHandshake::new(context)?,
                ReceiptSessionBinding {
                    endpoint: ReceiptEndpoint::Server,
                    local: cut.local.clone(),
                    connection,
                    server_pin: config.server_pin,
                    minimum_membership: config.minimum_membership,
                },
                cut.membership,
            )?;
            let response = session.send(cut.membership, cut.device_key)?;
            fresh(created)?;
            self.bound = Some((peer, connection));
            self.returned = Some((response.clone(), created, cut.membership.revision));
            self.pending = Some(ServerPending { session, created });
            Ok(response)
        })
    }
    /// Owner installs returned active session only after its exact Finished response is flushed.
    pub(crate) fn proof(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(ReceiptRecord, ReceiptSession), PeerError> {
        if self.delivery.queued() {
            self.invalidate();
            return Err(PeerError::Replay);
        }
        self.delivery.clear_prepared()?;
        self.returned = None;
        let ServerPending {
            mut session,
            created,
        } = self.pending.take().ok_or(PeerError::Replay)?;
        fresh(created)?;
        repo.require_context(&record.context)?;
        repo.with_current_read(|cut| {
            fresh(created)?;
            session.receive(record, peer, connection, cut.membership)?;
            let finished = session.send(cut.membership, cut.device_key)?;
            self.finished_digest = Some(session.handshake().context_digest()?);
            self.returned = Some((finished.clone(), created, cut.membership.revision));
            Ok((finished, session))
        })
    }
}
