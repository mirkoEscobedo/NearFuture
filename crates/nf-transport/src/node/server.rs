use super::{connection::Connection, local_current, membership};
use crate::{
    PeerError,
    identity::TransportIdentity,
    network::{PeerBehaviour, PeerBehaviourEvent},
    query::QueryServer,
    records::Lane,
    session::{ServerSession, SessionPolicy},
};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    swarm::{ConnectionId, SwarmEvent},
};
use nf_identity::{
    model::MembershipState,
    private_storage::{LocalIdentity, PrivateVault},
};
use nf_store::Store;
use std::time::{Duration, Instant};
#[derive(Debug)]
pub enum PeerServerEvent {
    Ready {
        control: Multiaddr,
        bulk: Multiaddr,
        peer: PeerId,
    },
    Progress,
    Rejected(PeerError),
}
/// Foreground owner. No background service, world writes or native game hooks are installed.
pub struct PeerServer {
    pub(super) control: Swarm<PeerBehaviour>,
    pub(super) bulk: Swarm<PeerBehaviour>,
    pub(super) local: LocalIdentity,
    pub(super) peer: PeerId,
    pub(super) policy: SessionPolicy,
    pub(super) store: Option<Store>,
    pub(super) query: Option<QueryServer>,
    pub(super) control_auth: Option<ServerSession>,
    pub(super) bulk_auth: Option<ServerSession>,
    pub(super) bulk_transfer: Option<crate::bulk::BulkServer>,
    pub(super) control_connection: Option<Connection>,
    pub(super) bulk_connection: Option<Connection>,
    control_addr: Option<Multiaddr>,
    bulk_addr: Option<Multiaddr>,
    announced: bool,
}
impl PeerServer {
    pub fn new(
        mut store: Store,
        vault: &PrivateVault,
        policy: SessionPolicy,
    ) -> Result<Self, PeerError> {
        policy.limits.validate()?;
        let identity = TransportIdentity::load(vault)?;
        let peer = identity.peer_id();
        let local = vault
            .load_identity(peer.to_bytes())
            .map_err(|_| PeerError::Storage)?;
        let state = membership(&mut store, policy)?;
        local_current(&state, &local, true)?;
        let mut control = identity.build_lane(Lane::Control)?;
        let mut bulk = identity.build_lane(Lane::Bulk)?;
        control
            .listen_on(
                "/ip4/127.0.0.1/tcp/0"
                    .parse()
                    .map_err(|_| PeerError::Offline)?,
            )
            .map_err(|_| PeerError::Offline)?;
        bulk.listen_on(
            "/ip4/127.0.0.1/tcp/0"
                .parse()
                .map_err(|_| PeerError::Offline)?,
        )
        .map_err(|_| PeerError::Offline)?;
        Ok(Self {
            control,
            bulk,
            local,
            peer,
            policy,
            store: Some(store),
            query: None,
            control_auth: None,
            bulk_auth: None,
            bulk_transfer: None,
            control_connection: None,
            bulk_connection: None,
            control_addr: None,
            bulk_addr: None,
            announced: false,
        })
    }
    pub(super) fn current(&mut self) -> Result<MembershipState, PeerError> {
        let store = if let Some(q) = &mut self.query {
            q.owner_store_mut()
        } else {
            self.store.as_mut().ok_or(PeerError::Storage)?
        };
        membership(store, self.policy)
    }
    pub(super) fn connection(&mut self, lane: Lane) -> &mut Option<Connection> {
        match lane {
            Lane::Control => &mut self.control_connection,
            Lane::Bulk => &mut self.bulk_connection,
        }
    }
    pub(super) fn swarm(&mut self, lane: Lane) -> &mut Swarm<PeerBehaviour> {
        match lane {
            Lane::Control => &mut self.control,
            Lane::Bulk => &mut self.bulk,
        }
    }
    pub(super) fn close(&mut self, lane: Lane, peer: PeerId, id: ConnectionId) {
        let owned = self
            .connection(lane)
            .as_ref()
            .is_some_and(|c| c.matches(peer, id));
        if owned {
            *self.connection(lane) = None;
            match lane {
                Lane::Control => {
                    self.control_auth = None;
                    if let Some(q) = self.query.take() {
                        self.store = Some(q.into_store());
                    }
                }
                Lane::Bulk => {
                    self.bulk_auth = None;
                    self.bulk_transfer = None;
                }
            }
        }
        let _ = self.swarm(lane).close_connection(id);
    }
    fn expire(&mut self, now: Instant) {
        for lane in [Lane::Control, Lane::Bulk] {
            let active = match lane {
                Lane::Control => self.query.is_some(),
                Lane::Bulk => self.bulk_transfer.is_some(),
            };
            let expired = self
                .connection(lane)
                .as_ref()
                .filter(|c| {
                    now.duration_since(c.last) >= Duration::from_secs(10)
                        || !active && now.duration_since(c.opened) >= Duration::from_secs(5)
                })
                .map(|c| (c.peer, c.id));
            if let Some((p, id)) = expired {
                self.close(lane, p, id);
            }
        }
    }
    pub async fn next(&mut self) -> Result<PeerServerEvent, PeerError> {
        self.expire(Instant::now());
        let (lane, event) = tokio::select! {e=self.control.select_next_some()=>(Lane::Control,e),e=self.bulk.select_next_some()=>(Lane::Bulk,e),_=tokio::time::sleep(Duration::from_millis(25))=>return Ok(PeerServerEvent::Progress)};
        if let SwarmEvent::NewListenAddr { address, .. } = event {
            let address = address.with(libp2p::multiaddr::Protocol::P2p(self.peer));
            match lane {
                Lane::Control => self.control_addr = Some(address),
                Lane::Bulk => self.bulk_addr = Some(address),
            };
            if !self.announced
                && let (Some(control), Some(bulk)) = (&self.control_addr, &self.bulk_addr)
            {
                self.announced = true;
                return Ok(PeerServerEvent::Ready {
                    control: control.clone(),
                    bulk: bulk.clone(),
                    peer: self.peer,
                });
            }
            return Ok(PeerServerEvent::Progress);
        }
        self.handle(lane, event)?;
        Ok(PeerServerEvent::Progress)
    }
    pub(super) fn handle(
        &mut self,
        lane: Lane,
        event: SwarmEvent<PeerBehaviourEvent>,
    ) -> Result<(), PeerError> {
        self.handle_event(lane, event)
    }
}
