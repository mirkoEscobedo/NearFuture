use crate::{
    AdmittedClient, AuthenticatedSession, Authenticator, FramePump, IpcError, LocalPrincipal,
    LoopbackListener, SessionConfig, SessionFence,
};
use nf_wire::generated as g;
use prost::Message;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
/// Background query seam. Writes and unregistered kernel outcomes must fail closed.
pub trait QueryPort {
    fn query(&mut self, query: g::QueryOperation) -> Result<g::OperationStatus, IpcError>;
    /// Bounded staging only, never game installation. Discard staged bytes on abort.
    fn stage_chunk(&mut self, _: crate::TransferPiece) -> Result<(), IpcError> {
        Err(IpcError::Unsupported)
    }
    fn abort_bulk(&mut self) {}
}
pub struct NoOperations;
impl QueryPort for NoOperations {
    fn query(&mut self, _: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        Err(IpcError::Unsupported)
    }
}
struct Connection {
    pump: FramePump,
    session: Option<AuthenticatedSession>,
    last_progress: Instant,
    bulk: Option<crate::BulkReceiver>,
    challenge: Option<crate::ServerChallenge>,
}
pub struct NodeServer<P> {
    listener: LoopbackListener,
    config: SessionConfig,
    principal: LocalPrincipal,
    authenticator: Option<Authenticator>,
    fence: SessionFence,
    connections: Vec<Connection>,
    capacity: usize,
    port: P,
    bulk_endpoint: bool,
}
impl<P: QueryPort> NodeServer<P> {
    pub fn bind(
        config: SessionConfig,
        principal: LocalPrincipal,
        port: P,
    ) -> Result<Self, IpcError> {
        config.validate()?;
        let reservation = 2 * (config.limits.control_frame_bytes as usize + 4) + 4096;
        let capacity = ((config.limits.inflight_bytes as usize / 2) / reservation)
            .min(config.limits.inflight_items as usize / 4)
            .min(8);
        if capacity == 0 {
            return Err(IpcError::Limit);
        }
        let fence = SessionFence::new(config.runtime_session)?;
        Ok(Self {
            listener: LoopbackListener::bind()?,
            config,
            principal,
            authenticator: None,
            fence,
            connections: Vec::with_capacity(capacity),
            capacity,
            port,
            bulk_endpoint: false,
        })
    }
    pub fn bind_bulk(
        config: SessionConfig,
        principal: LocalPrincipal,
        port: P,
    ) -> Result<Self, IpcError> {
        let mut node = Self::bind(config, principal, port)?;
        node.bulk_endpoint = true;
        node.capacity = 1;
        Ok(node)
    }
    pub fn address(&self) -> SocketAddr {
        self.listener.address()
    }
    pub fn install_authenticator(&mut self, authenticator: Authenticator) -> Result<(), IpcError> {
        if !self.connections.is_empty()
            || !authenticator.matches_config(
                &self.config,
                self.principal,
                if self.bulk_endpoint {
                    crate::EndpointRole::Bulk
                } else {
                    crate::EndpointRole::Control
                },
                self.address().port(),
            )
        {
            return Err(IpcError::SessionMismatch);
        }
        self.authenticator = Some(authenticator);
        Ok(())
    }
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
    /// Call before shutdown/load/save. No queued success or stale connection survives this fence.
    pub fn invalidate(&mut self) {
        self.fence.invalidate();
        self.port.abort_bulk();
        self.connections.clear();
        self.authenticator = None;
    }
    /// One bounded background turn; each connection receives <=4096 bytes in either direction.
    pub fn poll(&mut self, now: Instant) -> Result<(), IpcError> {
        let Some(auth) = &self.authenticator else {
            return Err(IpcError::ReadOnly);
        };
        if self.connections.len() < self.capacity
            && let Some(pump) = self
                .listener
                .try_accept((self.config.limits.control_frame_bytes as usize).min(4096))?
        {
            self.connections.push(Connection {
                pump,
                session: None,
                last_progress: now,
                bulk: None,
                challenge: None,
            });
        }
        let mut index = 0;
        while index < self.connections.len() {
            let c = &mut self.connections[index];
            let timeout = if c.session.is_some() {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(2)
            };
            if now.saturating_duration_since(c.last_progress) >= timeout {
                self.port.abort_bulk();
                self.connections.swap_remove(index);
                continue;
            }
            if c.pump.pending_write_bytes() != 0 {
                match c.pump.flush(4096) {
                    Ok(()) => {}
                    _ => {
                        self.port.abort_bulk();
                        self.connections.swap_remove(index);
                        continue;
                    }
                }
                index += 1;
                continue;
            }
            let frame = match c.pump.poll(4096, 4096) {
                Ok(None) => {
                    index += 1;
                    continue;
                }
                Ok(Some(body)) => body,
                Err(_) => {
                    self.port.abort_bulk();
                    self.connections.swap_remove(index);
                    continue;
                }
            };
            c.last_progress = now;
            let frame = zeroize::Zeroizing::new(frame);
            if let Some(receiver) = &mut c.bulk {
                match receiver.accept(&frame, &self.fence) {
                    Ok(piece) => {
                        let complete = piece.completion.is_some();
                        if self.port.stage_chunk(piece).is_err() {
                            self.port.abort_bulk();
                            self.connections.swap_remove(index);
                            continue;
                        }
                        if !complete {
                            index += 1;
                            continue;
                        }
                    }
                    Err(_) => {
                        self.port.abort_bulk();
                        self.connections.swap_remove(index);
                        continue;
                    }
                }
                let e = g::ControlEnvelope {
                    protocol_version: 1,
                    runtime_session: Some(g::RuntimeSession {
                        value: self.config.runtime_session,
                    }),
                    required: Some(required()),
                    body: Some(g::control_envelope::Body::Error(bounded_error(
                        IpcError::Unsupported,
                    ))),
                    transport: None,
                };
                if c.pump.send(&e.encode_to_vec()).is_err() {
                    self.port.abort_bulk();
                    self.connections.swap_remove(index);
                    continue;
                }
                index += 1;
                continue;
            }
            if c.session.is_none() {
                let response = if let Some(challenge) = c.challenge.take() {
                    match auth.finish(challenge, &frame) {
                        Ok((session, encoded)) => {
                            let reservation =
                                2 * (session.limits.control_frame_bytes as usize + 4) + 4096;
                            if (session.limits.inflight_bytes as usize) < reservation
                                || session.limits.inflight_items < 2
                            {
                                self.port.abort_bulk();
                                self.connections.swap_remove(index);
                                continue;
                            }
                            if self.bulk_endpoint {
                                let mut config = self.config.clone();
                                config.limits = session.limits;
                                c.bulk = Some(crate::BulkReceiver::new(config, self.principal)?);
                            }
                            c.pump.activate(&session)?;
                            c.session = Some(session);
                            encoded
                        }
                        Err(_) => {
                            self.port.abort_bulk();
                            self.connections.swap_remove(index);
                            continue;
                        }
                    }
                } else {
                    match auth.begin(&frame) {
                        Ok(challenge) => {
                            let response = challenge.challenge().to_vec();
                            c.challenge = Some(challenge);
                            response
                        }
                        Err(_) => {
                            self.port.abort_bulk();
                            self.connections.swap_remove(index);
                            continue;
                        }
                    }
                };
                if c.pump.send(&response).is_err() {
                    self.port.abort_bulk();
                    self.connections.swap_remove(index);
                    continue;
                }
                index += 1;
                continue;
            }
            let session = c.session.as_ref().ok_or(IpcError::ReadOnly)?;
            let response = match session.admit_client(&frame, self.principal, &self.fence) {
                Ok(AdmittedClient::Query(q)) => self
                    .port
                    .query(q)
                    .map(g::control_envelope::Body::OperationStatus),
                Ok(AdmittedClient::Intent(_)) => Err(IpcError::Unsupported),
                Err(e) => Err(e),
            };
            let body = response
                .unwrap_or_else(|error| g::control_envelope::Body::Error(bounded_error(error)));
            let envelope = g::ControlEnvelope {
                protocol_version: 1,
                runtime_session: Some(g::RuntimeSession {
                    value: self.config.runtime_session,
                }),
                required: Some(required()),
                body: Some(body),
                transport: None,
            };
            let encoded = envelope.encode_to_vec();
            if nf_wire::decode_control_with_limits(
                &encoded,
                crate::session::wire_limits(&session.limits),
            )
            .is_err()
                || c.pump.send(&encoded).is_err()
            {
                self.port.abort_bulk();
                self.connections.swap_remove(index);
                continue;
            }
            index += 1;
        }
        Ok(())
    }
}
fn required() -> g::RequiredSemantics {
    g::RequiredSemantics {
        capability_ids: vec![1],
        schema_ids: vec![1],
    }
}
pub(crate) fn bounded_error(error: IpcError) -> g::BoundedError {
    let (code, reason) = match error {
        IpcError::Unauthorized => (
            g::ErrorCode::Unauthorized,
            "Local principal is not authorized.",
        ),
        IpcError::HistoryMismatch => (g::ErrorCode::HistoryMismatch, "History does not match."),
        IpcError::SessionMismatch | IpcError::ReadOnly => (
            g::ErrorCode::SessionMismatch,
            "Session is unavailable; resynchronize.",
        ),
        IpcError::Backpressure => (g::ErrorCode::Backpressure, "Queue capacity is exhausted."),
        IpcError::Limit => (g::ErrorCode::LimitExceeded, "IPC resource limit exceeded."),
        _ => (
            g::ErrorCode::UnknownRequiredSemantic,
            "No registered operation response is available.",
        ),
    };
    g::BoundedError {
        code: code as i32,
        reason: reason.into(),
        unsupported_capability_ids: vec![],
        unsupported_schema_ids: vec![],
        retryable: matches!(error, IpcError::Backpressure | IpcError::ReadOnly),
    }
}
