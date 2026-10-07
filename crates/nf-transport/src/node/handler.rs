use super::{PeerServer, connection::Connection};
use crate::{
    PeerError,
    network::PeerBehaviourEvent,
    query::QueryServer,
    records::{Lane, PeerBody, PeerRecord, encode_body},
    session::ServerSession,
};
use libp2p::{
    PeerId,
    request_response::{Event, Message},
    swarm::{ConnectionId, SwarmEvent},
};
use std::time::Instant;
impl PeerServer {
    pub(super) fn handle_event(
        &mut self,
        lane: Lane,
        event: SwarmEvent<PeerBehaviourEvent>,
    ) -> Result<(), PeerError> {
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                if self.connection(lane).is_some() {
                    let _ = self.swarm(lane).close_connection(connection_id);
                } else {
                    *self.connection(lane) = Some(Connection::new(
                        peer_id,
                        connection_id,
                        lane,
                        Instant::now(),
                    )?);
                }
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                ..
            } => self.close(lane, peer_id, connection_id),
            SwarmEvent::ListenerError { .. } | SwarmEvent::ListenerClosed { .. } => {
                return Err(PeerError::Offline);
            }
            SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Request {
                        request_id,
                        request,
                        channel,
                    },
            })) => {
                let result = (|| {
                    let c = self
                        .connection(lane)
                        .as_mut()
                        .filter(|c| c.matches(peer, connection_id))
                        .ok_or(PeerError::Session)?;
                    encode_body(&request, lane, c.limits)?;
                    c.last = Instant::now();
                    let response = self.record(lane, request, peer, connection_id)?;
                    self.connection(lane)
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .reserve(lane, request_id, &response)?;
                    self.swarm(lane)
                        .behaviour_mut()
                        .messages
                        .send_response(channel, response)
                        .map_err(|_| PeerError::Offline)?;
                    Ok(())
                })();
                if let Err(e) = result {
                    self.close(lane, peer, connection_id);
                    if e == PeerError::Storage {
                        return Err(e);
                    }
                }
            }
            SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::ResponseSent {
                peer,
                connection_id,
                request_id,
            })) => {
                if let Some(c) = self
                    .connection(lane)
                    .as_mut()
                    .filter(|c| c.matches(peer, connection_id))
                    && c.flushed(request_id).is_err()
                {
                    self.close(lane, peer, connection_id);
                }
            }
            SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(
                Event::InboundFailure {
                    peer,
                    connection_id,
                    ..
                }
                | Event::OutboundFailure {
                    peer,
                    connection_id,
                    ..
                }
                | Event::Message {
                    peer,
                    connection_id,
                    ..
                },
            )) => self.close(lane, peer, connection_id),
            _ => {}
        };
        Ok(())
    }
    fn record(
        &mut self,
        lane: Lane,
        record: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
    ) -> Result<PeerRecord, PeerError> {
        match lane {
            Lane::Control => match record.body {
                PeerBody::Hello { .. } => {
                    if self.control_auth.is_some() || self.query.is_some() {
                        return Err(PeerError::Session);
                    }
                    let state = self.current()?;
                    let (auth, reply) = ServerSession::begin(
                        record,
                        peer,
                        id,
                        self.local.public.clone(),
                        self.peer,
                        lane,
                        self.policy,
                        &state,
                        &self.local.device_key,
                    )?;
                    self.control_connection
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .negotiate(lane, auth.limits())?;
                    self.control_auth = Some(auth);
                    Ok(reply)
                }
                PeerBody::ClientProof(_) => {
                    let mut auth = self.control_auth.take().ok_or(PeerError::Replay)?;
                    let state = self.current()?;
                    let reply =
                        auth.client_proof(record, peer, id, &state, &self.local.device_key)?;
                    let store = self.store.take().ok_or(PeerError::Storage)?;
                    self.query =
                        Some(QueryServer::new(auth, store).map_err(|_| PeerError::Storage)?);
                    Ok(reply)
                }
                PeerBody::BeginQuery { .. } => self
                    .query
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .begin(record, peer, id, Instant::now()),
                PeerBody::ProveQuery { .. } => self
                    .query
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .prove(record, peer, id, &self.local.device_key, Instant::now()),
                _ => Err(PeerError::Unsupported),
            },
            Lane::Bulk => match record.body {
                PeerBody::Hello { .. } => {
                    if self.bulk_auth.is_some() || self.bulk_transfer.is_some() {
                        return Err(PeerError::Session);
                    }
                    let state = self.current()?;
                    let (auth, reply) = ServerSession::begin(
                        record,
                        peer,
                        id,
                        self.local.public.clone(),
                        self.peer,
                        lane,
                        self.policy,
                        &state,
                        &self.local.device_key,
                    )?;
                    self.bulk_connection
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .negotiate(lane, auth.limits())?;
                    self.bulk_auth = Some(auth);
                    Ok(reply)
                }
                PeerBody::ClientProof(_) => {
                    let mut auth = self.bulk_auth.take().ok_or(PeerError::Replay)?;
                    let state = self.current()?;
                    let reply =
                        auth.client_proof(record, peer, id, &state, &self.local.device_key)?;
                    self.bulk_transfer = Some(crate::bulk::BulkServer::new(auth, &state)?);
                    Ok(reply)
                }
                PeerBody::BeginBulk { .. } => {
                    let state = self.current()?;
                    self.bulk_transfer
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .begin(record, peer, id, &state, Instant::now())
                }
                PeerBody::ProveBulk { .. } => {
                    let state = self.current()?;
                    self.bulk_transfer
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .prove(
                            record,
                            peer,
                            id,
                            &state,
                            &self.local.device_key,
                            Instant::now(),
                        )
                }
                PeerBody::BulkChunk { .. } => {
                    let state = self.current()?;
                    self.bulk_transfer
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .chunk(
                            record,
                            peer,
                            id,
                            &state,
                            &self.local.device_key,
                            Instant::now(),
                        )
                }
                _ => Err(PeerError::Unsupported),
            },
        }
    }
}
