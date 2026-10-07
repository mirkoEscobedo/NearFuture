use super::{local_current, membership};
use crate::{
    PeerError,
    auth::ServerPin,
    bulk::{BulkClient, BulkResult},
    identity::TransportIdentity,
    network::{PeerBehaviour, PeerBehaviourEvent},
    query::{QueryClient, QueryResult},
    records::{BulkDescriptor, Lane, PeerBody, PeerRecord},
    session::{ClientSession, SessionPolicy, fresh},
};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    multiaddr::Protocol,
    request_response::{Event, Message, OutboundRequestId},
    swarm::{ConnectionId, SwarmEvent},
};
use nf_contract::identity::RequestId;
use nf_identity::{
    model::MembershipState,
    private_storage::{LocalIdentity, PrivateVault},
};
use nf_store::Store;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
struct LaneOwner {
    swarm: Swarm<PeerBehaviour>,
    connected: Option<ConnectionId>,
    expected: Option<OutboundRequestId>,
    auth: Option<ClientSession>,
    hello: PeerRecord,
}
struct PairOwner<'a> {
    control: LaneOwner,
    bulk_lane: LaneOwner,
    store: Option<Store>,
    query: Option<QueryClient>,
    bulk: Option<BulkClient>,
    local: LocalIdentity,
    policy: SessionPolicy,
    pin: ServerPin,
    request: RequestId,
    bytes: &'a [u8],
    cursor: usize,
    chunk_size: usize,
    query_result: Option<QueryResult>,
    bulk_result: Option<BulkResult>,
}
/// One foreground authority for two physical lanes and one durable repository. At most one operation per lane.
#[allow(clippy::too_many_arguments)] // Explicit two-endpoint owner composition; each lane admits one operation.
pub async fn query_and_verify_bulk(
    store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    control: Multiaddr,
    bulk: Multiaddr,
    request: RequestId,
    bytes: &[u8],
) -> Result<(QueryResult, BulkResult), PeerError> {
    if bytes.is_empty() || bytes.len() > 1_048_576 {
        return Err(PeerError::Limit);
    }
    tokio::time::timeout(
        Duration::from_secs(15),
        run(store, vault, policy, pin, control, bulk, request, bytes),
    )
    .await
    .map_err(|_| PeerError::Offline)?
}
fn lane_owner(
    identity: &TransportIdentity,
    local: &LocalIdentity,
    policy: SessionPolicy,
    pin: ServerPin,
    lane: Lane,
    address: Multiaddr,
) -> Result<LaneOwner, PeerError> {
    let fields: Vec<_> = address.iter().collect();
    if !matches!(fields.as_slice(),[Protocol::Ip4(ip),Protocol::Tcp(port),Protocol::P2p(peer)]if ip.is_loopback()&&*port!=0&&*peer==pin.peer)
    {
        return Err(PeerError::Unauthorized);
    }
    let (auth, hello) =
        ClientSession::begin(local.public.clone(), identity.peer_id(), pin, lane, policy)?;
    let mut swarm = identity.build_lane(lane)?;
    swarm.dial(address).map_err(|_| PeerError::Offline)?;
    Ok(LaneOwner {
        swarm,
        connected: None,
        expected: None,
        auth: Some(auth),
        hello,
    })
}
#[allow(clippy::too_many_arguments)] // Explicit owner context and independent endpoints; no claimed wire authority.
async fn run(
    mut store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    control: Multiaddr,
    bulk: Multiaddr,
    request: RequestId,
    bytes: &[u8],
) -> Result<(QueryResult, BulkResult), PeerError> {
    if control == bulk {
        return Err(PeerError::Unauthorized);
    }
    let identity = TransportIdentity::load(vault)?;
    let local = vault
        .load_identity(identity.peer_id().to_bytes())
        .map_err(|_| PeerError::Storage)?;
    let state = membership(&mut store, policy)?;
    local_current(&state, &local, false)?;
    let control = lane_owner(&identity, &local, policy, pin, Lane::Control, control)?;
    let bulk_lane = lane_owner(&identity, &local, policy, pin, Lane::Bulk, bulk)?;
    let mut owner = PairOwner {
        control,
        bulk_lane,
        store: Some(store),
        query: None,
        bulk: None,
        local,
        policy,
        pin,
        request,
        bytes,
        cursor: 0,
        chunk_size: 0,
        query_result: None,
        bulk_result: None,
    };
    loop {
        let (lane, event) = tokio::select! {event=owner.control.swarm.select_next_some()=>(Lane::Control,event),event=owner.bulk_lane.swarm.select_next_some()=>(Lane::Bulk,event)};
        owner.event(lane, event)?;
        if owner.query_result.is_some() && owner.bulk_result.is_some() {
            return Ok((
                owner.query_result.take().ok_or(PeerError::Session)?,
                owner.bulk_result.take().ok_or(PeerError::Session)?,
            ));
        }
    }
}
impl PairOwner<'_> {
    fn lane(&mut self, lane: Lane) -> &mut LaneOwner {
        match lane {
            Lane::Control => &mut self.control,
            Lane::Bulk => &mut self.bulk_lane,
        }
    }
    fn current(&mut self) -> Result<MembershipState, PeerError> {
        let s = if let Some(q) = &mut self.query {
            q.owner_store_mut()
        } else {
            self.store.as_mut().ok_or(PeerError::Session)?
        };
        membership(s, self.policy)
    }
    fn event(
        &mut self,
        lane: Lane,
        event: SwarmEvent<PeerBehaviourEvent>,
    ) -> Result<(), PeerError> {
        let complete = match lane {
            Lane::Control => self.query_result.is_some(),
            Lane::Bulk => self.bulk_result.is_some(),
        };
        if complete {
            return Ok(());
        }
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                if peer_id != self.pin.peer || self.lane(lane).connected.is_some() {
                    return Err(PeerError::Unauthorized);
                }
                let pin = self.pin.peer;
                let l = self.lane(lane);
                l.connected = Some(connection_id);
                l.expected = Some(
                    l.swarm
                        .behaviour_mut()
                        .messages
                        .send_request(&pin, l.hello.clone()),
                );
            }
            SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Response {
                        request_id,
                        response,
                    },
            })) => {
                if Some(request_id) != self.lane(lane).expected.take()
                    || peer != self.pin.peer
                    || Some(connection_id) != self.lane(lane).connected
                {
                    return Err(PeerError::Session);
                }
                let state = self.current()?;
                let now = Instant::now();
                let next = self.response(lane, response, peer, connection_id, &state, now)?;
                if let Some(next) = next {
                    let pin = self.pin.peer;
                    self.lane(lane).expected = Some(
                        self.lane(lane)
                            .swarm
                            .behaviour_mut()
                            .messages
                            .send_request(&pin, next),
                    );
                }
            }
            SwarmEvent::OutgoingConnectionError { .. }
            | SwarmEvent::ConnectionClosed { .. }
            | SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(
                Event::OutboundFailure { .. }
                | Event::InboundFailure { .. }
                | Event::Message { .. },
            )) => return Err(PeerError::Offline),
            _ => {}
        }
        Ok(())
    }
    fn response(
        &mut self,
        lane: Lane,
        r: PeerRecord,
        peer: PeerId,
        id: ConnectionId,
        state: &MembershipState,
        now: Instant,
    ) -> Result<Option<PeerRecord>, PeerError> {
        if matches!(r.body, PeerBody::ServerHello { .. }) {
            let key = &self.local.device_key;
            let auth = match lane {
                Lane::Control => &mut self.control.auth,
                Lane::Bulk => &mut self.bulk_lane.auth,
            };
            return Ok(Some(
                auth.as_mut()
                    .ok_or(PeerError::Session)?
                    .server_hello(r, peer, id, state, key)?,
            ));
        }
        if matches!(r.body, PeerBody::Finished(_)) {
            let mut active = self.lane(lane).auth.take().ok_or(PeerError::Session)?;
            active.finished(r, peer, id, state)?;
            return match lane {
                Lane::Control => {
                    let mut q =
                        QueryClient::new(active, self.store.take().ok_or(PeerError::Session)?)?;
                    let r = q.begin(self.request, now)?;
                    self.query = Some(q);
                    Ok(Some(r))
                }
                Lane::Bulk => {
                    self.chunk_size = active.binding()?.context.selected.chunk_bytes as usize;
                    let d = BulkDescriptor {
                        transfer: fresh()?,
                        total: self.bytes.len() as u64,
                        chunks: self
                            .bytes
                            .len()
                            .div_ceil(self.chunk_size)
                            .try_into()
                            .map_err(|_| PeerError::Limit)?,
                        digest: Sha256::digest(self.bytes).into(),
                    };
                    let mut b = BulkClient::new(active, state)?;
                    let r = b.begin(d, state, now)?;
                    self.bulk = Some(b);
                    Ok(Some(r))
                }
            };
        }
        match lane {
            Lane::Control => match r.body {
                PeerBody::QueryChallenge { .. } => Ok(Some(
                    self.query.as_mut().ok_or(PeerError::Session)?.challenge(
                        r,
                        peer,
                        id,
                        &self.local.device_key,
                        now,
                    )?,
                )),
                PeerBody::RetainedStatus { .. } | PeerBody::Unsupported { .. } => {
                    self.query_result = Some(
                        self.query
                            .as_mut()
                            .ok_or(PeerError::Session)?
                            .reply(r, peer, id, now)?,
                    );
                    Ok(None)
                }
                _ => Err(PeerError::Unsupported),
            },
            Lane::Bulk => {
                match r.body {
                    PeerBody::BulkChallenge { .. } => {
                        return Ok(Some(
                            self.bulk.as_mut().ok_or(PeerError::Session)?.challenge(
                                r,
                                peer,
                                id,
                                state,
                                &self.local.device_key,
                                now,
                            )?,
                        ));
                    }
                    PeerBody::BulkReady { .. } => self
                        .bulk
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .ready(r, peer, id, state, now)?,
                    PeerBody::BulkProgress { .. } | PeerBody::BulkVerified { .. } => {
                        let result = self
                            .bulk
                            .as_mut()
                            .ok_or(PeerError::Session)?
                            .progress(r, peer, id, state, now)?;
                        if matches!(result, BulkResult::Verified { .. }) {
                            self.bulk_result = Some(result);
                            return Ok(None);
                        }
                    }
                    _ => return Err(PeerError::Unsupported),
                }
                let end = self.bytes.len().min(self.cursor + self.chunk_size);
                let r = self.bulk.as_mut().ok_or(PeerError::Session)?.chunk(
                    &self.bytes[self.cursor..end],
                    state,
                    now,
                )?;
                self.cursor = end;
                Ok(Some(r))
            }
        }
    }
}
