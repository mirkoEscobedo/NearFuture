use super::*;
use crate::notification::NotifyBody;
use crate::notification_effects::network::builder::NotifyBehaviourEvent;
use libp2p::{
    request_response::{Event, Message, ResponseChannel},
    swarm::SwarmEvent,
};
impl NotifyLane {
    pub(super) fn event(
        &mut self,
        event: SwarmEvent<NotifyBehaviourEvent>,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyLaneEvent>, PeerError> {
        match event {
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } if self.listener == Some(listener_id) => {
                Ok(Some(NotifyLaneEvent::Listening(address)))
            }
            SwarmEvent::IncomingConnection { .. } => {
                if self.connection.is_none() && self.pending.is_none() {
                    self.start_pending();
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
                    && peer_id != self.binding.server_pin.peer
                {
                    return Err(PeerError::Unauthorized);
                }
                self.connection = Some((peer_id, connection_id));
                self.start_pending();
                if let Endpoint::Client { handshake, .. } = &mut self.endpoint {
                    let local_peer = *self
                        .swarm
                        .as_ref()
                        .ok_or(PeerError::Offline)?
                        .local_peer_id();
                    let (h, hello) = NotifyClientHandshake::begin(
                        self.local.clone(),
                        local_peer,
                        self.binding.server_pin,
                        self.policy,
                    )?;
                    *handshake = Some(h);
                    self.queue_request(hello, peer_id, connection_id, repo)?;
                }
                Ok(None)
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                ..
            } if self.connection == Some((peer_id, connection_id)) => {
                self.shutdown();
                Ok(Some(NotifyLaneEvent::Closed))
            }
            SwarmEvent::Behaviour(NotifyBehaviourEvent::Messages(Event::Message {
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
                        super::super::network::NotifyRequest::Record(record) => {
                            self.request(*record, peer, connection_id, request_id, channel, repo)
                        }
                        super::super::network::NotifyRequest::Rejected => {
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
            SwarmEvent::Behaviour(NotifyBehaviourEvent::Messages(Event::ResponseSent {
                peer,
                connection_id,
                request_id,
            })) => self.sent(peer, connection_id, request_id, repo),
            // Codec errors happen before a decoded Message. Any such failure on this owned
            // connection burns pending challenges and closes, regardless of its new backend ID.
            SwarmEvent::Behaviour(NotifyBehaviourEvent::Messages(
                Event::InboundFailure {
                    peer,
                    connection_id,
                    ..
                }
                | Event::OutboundFailure {
                    peer,
                    connection_id,
                    ..
                },
            )) if self.connection == Some((peer, connection_id)) => Err(PeerError::Offline),
            SwarmEvent::OutgoingConnectionError { .. }
            | SwarmEvent::ListenerError { .. }
            | SwarmEvent::ListenerClosed { .. } => Err(PeerError::Offline),
            _ => Ok(None),
        }
    }
    fn request(
        &mut self,
        r: NotifyRecord,
        peer: PeerId,
        id: ConnectionId,
        request: InboundRequestId,
        channel: ResponseChannel<NotifyRecord>,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyLaneEvent>, PeerError> {
        if self.outbound.is_some() || self.inbound.is_some() {
            return Err(PeerError::Backpressure);
        }
        let local_peer = *self
            .swarm
            .as_ref()
            .ok_or(PeerError::Offline)?
            .local_peer_id();
        self.live_cut()?;
        let response = match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                session,
                broker,
            } => match &r.body {
                NotifyBody::Hello { .. }
                    if handshake.is_none() && session.is_none() && broker.is_none() =>
                {
                    let deadline = self.pending.as_ref().ok_or(PeerError::Replay)?;
                    let (h, reply) = repo.with_current_read(|cut| {
                        if deadline.expired_now() {
                            return Err(PeerError::Replay);
                        }
                        NotifyServerHandshake::begin(
                            r,
                            peer,
                            id,
                            cut.local.clone(),
                            local_peer,
                            self.policy,
                            cut.membership,
                            cut.device_key,
                        )
                    })?;
                    *handshake = Some(h);
                    reply
                }
                NotifyBody::ClientProof(_) if session.is_none() && broker.is_none() => {
                    let h = handshake.as_mut().ok_or(PeerError::Replay)?;
                    let (s, reply) = repo.with_current_read(|cut| {
                        h.client_proof(r, peer, id, cut.membership, cut.device_key)
                    })?;
                    *session = Some(s);
                    reply
                }
                NotifyBody::BeginSubscribe { .. } => {
                    let b = broker.as_mut().ok_or(PeerError::Session)?;
                    b.begin_subscription(r, peer, id, repo)?
                }
                NotifyBody::ProveSubscribe { .. } => {
                    let b = broker.as_mut().ok_or(PeerError::Session)?;
                    {
                        #[cfg(test)]
                        completion_test::before_prove(b.pending_setup_end_for_test()?);
                        b.prove_subscription(r, peer, id, repo)?
                    }
                }
                _ => return Err(PeerError::Replay),
            },
            Endpoint::Client {
                subscriber: Some(s),
                ..
            } => {
                if !matches!(r.body, NotifyBody::Notice { .. }) {
                    return Err(PeerError::Replay);
                }
                s.notice(r, peer, id, repo)?
            }
            _ => return Err(PeerError::Session),
        };
        let dirty = matches!(response.body, NotifyBody::NoticeAck { .. });
        self.queue_response(response, peer, id, request, channel, repo)?;
        self.start_pending();
        Ok(dirty.then_some(NotifyLaneEvent::Dirty))
    }
    fn response(
        &mut self,
        r: NotifyRecord,
        peer: PeerId,
        id: ConnectionId,
        request: OutboundRequestId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyLaneEvent>, PeerError> {
        if !self
            .outbound
            .as_ref()
            .is_some_and(|o| o.id == request && o.peer == peer && o.connection == id)
        {
            return Err(PeerError::Replay);
        }
        let owner_end = self.completion_end()?;
        let held = self.outbound.take().ok_or(PeerError::Replay)?;
        match &mut self.endpoint {
            Endpoint::Client {
                handshake,
                subscriber,
                original,
                lifetime,
                ..
            } => match &r.body {
                NotifyBody::ServerHello { .. } if subscriber.is_none() => {
                    let h = handshake.as_mut().ok_or(PeerError::Replay)?;
                    h.complete_emission(held.emission, peer, id, repo)?;
                    let proof = repo.with_current_read(|cut| {
                        h.server_hello(r, peer, id, cut.membership, cut.device_key)
                    })?;
                    self.queue_request(proof, peer, id, repo)?;
                }
                NotifyBody::Finished(_) if subscriber.is_none() => {
                    let h = handshake.as_mut().ok_or(PeerError::Replay)?;
                    h.complete_emission(held.emission, peer, id, repo)?;
                    let session =
                        repo.with_current_read(|cut| h.finished(r, peer, id, cut.membership))?;
                    let mut s = NotifySubscriber::new(session)?;
                    let started = Instant::now();
                    let begin = s.begin_subscription(original.clone(), *lifetime, repo)?;
                    self.lifetime = Some(Deadline::at(
                        started + Duration::from_secs(u64::from(*lifetime)),
                    ));
                    *subscriber = Some(s);
                    self.queue_request(begin, peer, id, repo)?;
                }
                NotifyBody::SubscribeChallenge { .. } => {
                    let s = subscriber.as_mut().ok_or(PeerError::Session)?;
                    s.live_output()?;
                    s.session.remote_from_repo(peer, id, repo)?;
                    s.live_output()?;
                    s.session.flow.finish_emission(held.emission)?;
                    let proof = s.challenge(r, peer, id, repo)?;
                    self.queue_request(proof, peer, id, repo)?;
                }
                NotifyBody::Subscribed { .. } => {
                    let s = subscriber.as_mut().ok_or(PeerError::Session)?;
                    let completion_end = owner_end.min(s.subscription_completion_end()?);
                    s.subscribed(r, peer, id, repo)?;
                    #[cfg(test)]
                    completion_test::at(completion_test::Cut::ClientSubscribed, 0, || {
                        s.session.remote_from_repo(peer, id, repo)
                    })?;
                    #[cfg(not(test))]
                    s.session.remote_from_repo(peer, id, repo)?;
                    s.active_live()?;
                    super::super::until_cut(completion_end)?;
                    s.session.flow.finish_emission(held.emission)?;
                    self.pending = None;
                    return Ok(Some(NotifyLaneEvent::Subscribed));
                }
                _ => return Err(PeerError::Replay),
            },
            Endpoint::Server {
                broker: Some(b), ..
            } => {
                b.accept_ack(r, peer, id, repo)?;
                #[cfg(test)]
                completion_test::at(completion_test::Cut::ServerAck, 0, || {
                    b.complete_emission_before(held.emission, peer, id, repo, owner_end)
                })?;
                #[cfg(not(test))]
                b.complete_emission_before(held.emission, peer, id, repo, owner_end)?;
                self.pending = None;
                return Ok(None);
            }
            _ => return Err(PeerError::Session),
        }
        self.start_pending();
        Ok(None)
    }
    fn sent(
        &mut self,
        peer: PeerId,
        id: ConnectionId,
        request: InboundRequestId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyLaneEvent>, PeerError> {
        if !self
            .inbound
            .as_ref()
            .is_some_and(|r| r.id == request && r.peer == peer && r.connection == id)
        {
            return Err(PeerError::Replay);
        }
        let held = self.inbound.take().ok_or(PeerError::Replay)?;
        let finished = matches!(held.emission.record().body, NotifyBody::Finished(_));
        let subscribed = matches!(held.emission.record().body, NotifyBody::Subscribed { .. });
        let ack = matches!(held.emission.record().body, NotifyBody::NoticeAck { .. });
        #[cfg(test)]
        if subscribed {
            completion_test::owner_end(self.pending.as_ref().ok_or(PeerError::Replay)?.end());
            completion_test::at(completion_test::Cut::ServerSubscribed, 2, || {
                self.complete(held.emission, peer, id, repo)
            })?;
        } else {
            self.complete(held.emission, peer, id, repo)?;
        }
        #[cfg(not(test))]
        self.complete(held.emission, peer, id, repo)?;
        if finished {
            let Endpoint::Server {
                session, broker, ..
            } = &mut self.endpoint
            else {
                return Err(PeerError::Session);
            };
            *broker = Some(NotifyBroker::new(session.take().ok_or(PeerError::Replay)?)?);
        }
        if subscribed {
            let Endpoint::Server {
                broker: Some(b), ..
            } = &self.endpoint
            else {
                return Err(PeerError::Session);
            };
            self.lifetime = Some(Deadline::after(b.remaining()?));
            self.pending = None;
            self.observe = Some(Box::pin(tokio::time::sleep(Duration::from_millis(100))));
            return Ok(Some(NotifyLaneEvent::SubscriptionResponseSent {
                peer,
                connection: id,
                request,
            }));
        }
        if ack {
            self.pending = None;
        }
        Ok(None)
    }
}
