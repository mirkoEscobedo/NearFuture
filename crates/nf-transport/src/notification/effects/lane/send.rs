use super::*;
use libp2p::request_response::ResponseChannel;
impl NotifyLane {
    pub(super) fn completion_end(&self) -> Result<Instant, PeerError> {
        self.live_cut()?;
        [&self.pending, &self.lifetime]
            .into_iter()
            .filter_map(|d| d.as_ref().map(Deadline::end))
            .min()
            .ok_or(PeerError::Replay)
    }
    pub(super) fn live_cut(&self) -> Result<(), PeerError> {
        if [&self.pending, &self.lifetime]
            .into_iter()
            .any(|d| d.as_ref().is_some_and(Deadline::expired_now))
        {
            return Err(PeerError::Replay);
        }
        Ok(())
    }
    fn prepare(
        &mut self,
        r: NotifyRecord,
        peer: PeerId,
        id: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyEmission, PeerError> {
        self.guard(repo)?;
        match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                session,
                broker,
            } => {
                if let Some(b) = broker {
                    return b.prepare_output(r, peer, id, repo);
                }
                if let Some(s) = session {
                    return s.prepare_setup_output(r, peer, id, repo);
                }
                handshake
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .prepare_output(r, peer, id, repo)
            }
            Endpoint::Client {
                handshake,
                subscriber,
                ..
            } => {
                if let Some(s) = subscriber {
                    s.live_output()?;
                    let emission = s.session.prepare_output(r, peer, id, repo)?;
                    s.live_output()?;
                    return Ok(emission);
                }
                handshake
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .prepare_output(r, peer, id, repo)
            }
        }
    }
    pub(super) fn complete(
        &mut self,
        e: NotifyEmission,
        peer: PeerId,
        id: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let end = self.completion_end()?;
        self.guard(repo)?;
        match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                session,
                broker,
            } => {
                if let Some(b) = broker {
                    return b.complete_emission_before(e, peer, id, repo, end);
                }
                if let Some(s) = session {
                    return s.complete_setup_emission(e, peer, id, repo);
                }
                handshake
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .complete_emission(e, peer, id, repo)
            }
            Endpoint::Client {
                handshake,
                subscriber,
                ..
            } => {
                if let Some(s) = subscriber {
                    s.live_output()?;
                    s.session.remote_from_repo(peer, id, repo)?;
                    s.live_output()?;
                    super::super::until_cut(end)?;
                    return s.session.flow.finish_emission(e).map(|_| ());
                }
                handshake
                    .as_mut()
                    .ok_or(PeerError::Session)?
                    .complete_emission(e, peer, id, repo)
            }
        }
    }
    pub(super) fn queue_request(
        &mut self,
        r: NotifyRecord,
        peer: PeerId,
        id: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        if self.outbound.is_some() || self.inbound.is_some() {
            return Err(PeerError::Backpressure);
        }
        let emission = self.prepare(r, peer, id, repo)?;
        self.live_cut()?;
        let request = self
            .swarm
            .as_mut()
            .ok_or(PeerError::Offline)?
            .behaviour_mut()
            .messages
            .send_request(
                &peer,
                super::super::network::NotifyRequest::Record(Box::new(emission.record().clone())),
            );
        self.outbound = Some(Outbound {
            id: request,
            peer,
            connection: id,
            emission,
        });
        Ok(())
    }
    pub(super) fn queue_response(
        &mut self,
        r: NotifyRecord,
        peer: PeerId,
        id: ConnectionId,
        request: InboundRequestId,
        channel: ResponseChannel<NotifyRecord>,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        if self.outbound.is_some() || self.inbound.is_some() {
            return Err(PeerError::Backpressure);
        }
        let emission = self.prepare(r, peer, id, repo)?;
        self.live_cut()?;
        self.swarm
            .as_mut()
            .ok_or(PeerError::Offline)?
            .behaviour_mut()
            .messages
            .send_response(channel, emission.record().clone())
            .map_err(|_| PeerError::Offline)?;
        self.inbound = Some(Inbound {
            id: request,
            peer,
            connection: id,
            emission,
        });
        Ok(())
    }
}
