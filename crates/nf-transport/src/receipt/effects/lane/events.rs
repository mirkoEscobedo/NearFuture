use super::*;
use crate::receipt::{ReceiptBody, ReceiptRecord};
use libp2p::{
    request_response::{Event, InboundRequestId, Message, OutboundRequestId, ResponseChannel},
    swarm::SwarmEvent,
};
impl ReceiptLane {
    pub(super) fn event(
        &mut self,
        event: SwarmEvent<ReceiptBehaviourEvent>,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<ReceiptLaneEvent>, PeerError> {
        match event {
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } if self.listener == Some(listener_id) => {
                Ok(Some(ReceiptLaneEvent::Listening(address)))
            }
            SwarmEvent::IncomingConnection { .. } => {
                if self.connection.is_none() && self.deadline.is_none() {
                    self.start_deadline();
                }
                Ok(None)
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                self.dialing = false;
                if self.connection.is_some() {
                    self.swarm
                        .as_mut()
                        .ok_or(PeerError::Offline)?
                        .close_connection(connection_id);
                    return Ok(None);
                }
                if matches!(self.endpoint, Endpoint::Client { .. })
                    && peer_id != repo.config().server_pin.peer
                {
                    self.swarm
                        .as_mut()
                        .ok_or(PeerError::Offline)?
                        .close_connection(connection_id);
                    return Err(PeerError::Unauthorized);
                }
                self.connection = Some((peer_id, connection_id));
                self.start_deadline();
                if let Endpoint::Client { handshake, .. } = &mut self.endpoint {
                    let hello = handshake.hello(repo)?;
                    handshake.queue_request(
                        hello,
                        peer_id,
                        connection_id,
                        self.swarm.as_mut().ok_or(PeerError::Offline)?,
                        repo,
                    )?;
                }
                Ok(None)
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                ..
            } if self.connection == Some((peer_id, connection_id)) => {
                self.connection = None;
                self.reset();
                Ok(Some(ReceiptLaneEvent::Closed { peer: peer_id }))
            }
            SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(Event::Message {
                peer,
                connection_id,
                message,
            })) => {
                if self.connection != Some((peer, connection_id)) {
                    return Err(PeerError::Session);
                }
                match message {
                    Message::Request {
                        request_id,
                        request,
                        channel,
                    } => match request {
                        ReceiptRequest::Record(record) => {
                            self.request(*record, peer, connection_id, request_id, channel, repo)
                        }
                        ReceiptRequest::Rejected => {
                            drop(channel);
                            Err(PeerError::Malformed)
                        }
                    },
                    Message::Response {
                        request_id,
                        response,
                    } => self.response(response, peer, connection_id, request_id, repo),
                }
            }
            SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(Event::ResponseSent {
                peer,
                connection_id,
                request_id,
            })) => self.sent(peer, connection_id, request_id, repo),
            SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(Event::InboundFailure {
                peer,
                connection_id,
                request_id,
                ..
            })) => {
                if let Endpoint::Server {
                    handshake,
                    operation,
                } = &mut self.endpoint
                {
                    if handshake.owns_response(request_id, peer, connection_id) {
                        handshake.delivery_failure(request_id, peer, connection_id)?;
                        return Err(PeerError::Offline);
                    }
                    if let Some(op) = operation
                        && op.owns_response(request_id, peer, connection_id)
                    {
                        op.delivery_failure(request_id, peer, connection_id)?;
                        return Err(PeerError::Offline);
                    }
                }
                // Registered inbound failures consume the exact owned physical session.
                // Consume the exact owned physical session; never reinterpret this as completion.
                if self.connection == Some((peer, connection_id)) {
                    Err(PeerError::Offline)
                } else {
                    Ok(None)
                }
            }
            SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(Event::OutboundFailure {
                peer,
                connection_id,
                request_id,
                ..
            })) => {
                if let Endpoint::Client {
                    handshake,
                    operation,
                    ..
                } = &mut self.endpoint
                {
                    if handshake.owns_request(request_id, peer, connection_id) {
                        handshake.delivery_failure(request_id, peer, connection_id)?;
                        return Err(PeerError::Offline);
                    }
                    if let Some(op) = operation
                        && op.owns_request(request_id, peer, connection_id)
                    {
                        op.delivery_failure(request_id, peer, connection_id)?;
                        return Err(PeerError::Offline);
                    }
                }
                Ok(None)
            }
            SwarmEvent::OutgoingConnectionError { .. } => {
                self.dialing = false;
                Err(PeerError::Offline)
            }
            SwarmEvent::ListenerError { .. } | SwarmEvent::ListenerClosed { .. } => {
                Err(PeerError::Offline)
            }
            _ => Ok(None),
        }
    }
    fn request(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        id: ConnectionId,
        request: InboundRequestId,
        channel: ResponseChannel<ReceiptRecord>,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<ReceiptLaneEvent>, PeerError> {
        if matches!(record.body, ReceiptBody::Begin { .. }) {
            if self.deadline.is_some() {
                return Err(PeerError::Backpressure);
            }
            self.start_deadline();
        }
        let Endpoint::Server {
            handshake,
            operation,
        } = &mut self.endpoint
        else {
            return Err(PeerError::Unsupported);
        };
        match &record.body {
            ReceiptBody::Hello { .. } if operation.is_none() => {
                let reply = handshake.begin(record, peer, id, repo)?;
                handshake.queue_response(
                    reply,
                    None,
                    InboundDelivery {
                        peer,
                        connection: id,
                        channel,
                        request,
                    },
                    self.swarm.as_mut().ok_or(PeerError::Offline)?,
                    repo,
                )?;
            }
            ReceiptBody::ClientProof(_) if operation.is_none() => {
                let (reply, active) = handshake.proof(record, peer, id, repo)?;
                handshake.queue_response(
                    reply,
                    Some(active),
                    InboundDelivery {
                        peer,
                        connection: id,
                        channel,
                        request,
                    },
                    self.swarm.as_mut().ok_or(PeerError::Offline)?,
                    repo,
                )?;
            }
            ReceiptBody::Begin { .. } => {
                let op = operation.as_mut().ok_or(PeerError::Session)?;
                let reply = op.begin(record, peer, id, repo)?;
                op.queue_response(
                    reply,
                    InboundDelivery {
                        peer,
                        connection: id,
                        channel,
                        request,
                    },
                    self.swarm.as_mut().ok_or(PeerError::Offline)?,
                    repo,
                )?;
            }
            ReceiptBody::Prove { .. } => {
                let op = operation.as_mut().ok_or(PeerError::Session)?;
                let reply = op.prove(record, peer, id, repo)?;
                op.queue_response(
                    reply,
                    InboundDelivery {
                        peer,
                        connection: id,
                        channel,
                        request,
                    },
                    self.swarm.as_mut().ok_or(PeerError::Offline)?,
                    repo,
                )?;
            }
            _ => return Err(PeerError::Unsupported),
        }
        Ok(None)
    }
    fn response(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        id: ConnectionId,
        request: OutboundRequestId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<ReceiptLaneEvent>, PeerError> {
        let Endpoint::Client {
            handshake,
            operation,
            slot,
            original,
            minimum,
        } = &mut self.endpoint
        else {
            return Err(PeerError::Unsupported);
        };
        if handshake.owns_request(request, peer, id) {
            handshake.response_received(request, peer, id, repo)?;
            match record.body {
                ReceiptBody::ServerHello { .. } => {
                    let proof = handshake.challenge(record, peer, id, repo)?;
                    handshake.queue_request(
                        proof,
                        peer,
                        id,
                        self.swarm.as_mut().ok_or(PeerError::Offline)?,
                        repo,
                    )?;
                    Ok(None)
                }
                ReceiptBody::Finished(_) => {
                    let session = handshake.finished(record, peer, id, repo)?;
                    self.active = Some(policy::ActivePolicy::capture(&session, repo)?);
                    let session = session.into_session();
                    *operation = Some(Box::new(ReceiptOperationClient::new(
                        session,
                        id,
                        original.as_ref().clone(),
                        *minimum,
                    )?));
                    self.deadline = None;
                    Ok(Some(ReceiptLaneEvent::Authenticated { peer }))
                }
                _ => Err(PeerError::Malformed),
            }
        } else if let Some(op) = operation
            && op.owns_request(request, peer, id)
        {
            op.response_received(request, peer, id, repo)?;
            match record.body {
                ReceiptBody::Challenge { .. } => {
                    let proof = op.challenge(record, peer, id, repo)?;
                    op.queue_request(
                        proof,
                        peer,
                        id,
                        self.swarm.as_mut().ok_or(PeerError::Offline)?,
                        repo,
                    )?;
                    Ok(None)
                }
                ReceiptBody::Status { .. } | ReceiptBody::Unsupported { .. } => {
                    let verified = op.reply(record, peer, id, repo)?;
                    let context = verified.context;
                    let original = verified.original.clone();
                    let created = verified.created;
                    let revision = verified.revision;
                    let accepted = repo.accept_verified(*slot, verified)?;
                    let correlation = self.query.take().map(|binding| binding.nonce);
                    let completion = match &accepted {
                        AdmittedReceipt::Status(status) => Some(Box::new(ReceiptCompletion {
                            correlation,
                            slot: *slot,
                            original,
                            context,
                            current: status.current,
                            revision,
                            created,
                        })),
                        AdmittedReceipt::Unsupported(_) => None,
                    };
                    self.deadline = None;
                    Ok(Some(ReceiptLaneEvent::Accepted {
                        outcome: accepted,
                        completion,
                    }))
                }
                _ => Err(PeerError::Malformed),
            }
        } else {
            Ok(None)
        } // Unmatched backend IDs never release occupied custody.
    }
    fn sent(
        &mut self,
        peer: PeerId,
        id: ConnectionId,
        request: InboundRequestId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<ReceiptLaneEvent>, PeerError> {
        let Endpoint::Server {
            handshake,
            operation,
        } = &mut self.endpoint
        else {
            return Ok(None);
        };
        if handshake.owns_response(request, peer, id) {
            if let Some(session) = handshake.response_sent(request, peer, id, repo)? {
                #[cfg(test)]
                let _boundary = super::activation_test::before_capture(repo)?;
                self.active = Some(policy::ActivePolicy::capture(&session, repo)?);
                let session = session.into_session();
                *operation = Some(Box::new(ReceiptOperationServer::new(session)?));
                self.deadline = None;
                return Ok(Some(ReceiptLaneEvent::Authenticated { peer }));
            }
        } else if let Some(op) = operation
            && op.owns_response(request, peer, id)
        {
            op.response_sent(request, peer, id, repo)?;
            if op.pending_delivery().0 == 0 && self.deadline.is_some() {
                // Challenge completion keeps the original operation deadline; terminal completion ends it.
                if op.awaiting_proof() {
                    return Ok(None);
                }
                self.deadline = None;
            }
        }
        Ok(None)
    }
}
