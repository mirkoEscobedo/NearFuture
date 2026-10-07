use super::{local_current, membership};
use crate::{
    PeerError,
    auth::ServerPin,
    bulk::{BulkClient, BulkResult},
    identity::TransportIdentity,
    network::PeerBehaviourEvent,
    records::{BulkDescriptor, Lane, PeerBody},
    session::{ClientSession, SessionPolicy, fresh},
};
use futures::StreamExt;
use libp2p::{
    Multiaddr,
    multiaddr::Protocol,
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_identity::private_storage::PrivateVault;
use nf_store::Store;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
/// Opaque byte digest only. No semantic decoder, installation, history sync or durable command acknowledgement.
pub async fn verify_bulk(
    store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    address: Multiaddr,
    bytes: &[u8],
) -> Result<BulkResult, PeerError> {
    if bytes.is_empty() || bytes.len() > 1_048_576 {
        return Err(PeerError::Limit);
    }
    tokio::time::timeout(
        Duration::from_secs(15),
        inner(store, vault, policy, pin, address, bytes),
    )
    .await
    .map_err(|_| PeerError::Offline)?
}
async fn inner(
    mut store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    address: Multiaddr,
    bytes: &[u8],
) -> Result<BulkResult, PeerError> {
    let fields: Vec<_> = address.iter().collect();
    if !matches!(fields.as_slice(),[Protocol::Ip4(ip),Protocol::Tcp(port),Protocol::P2p(peer)]if ip.is_loopback()&&*port!=0&&*peer==pin.peer)
    {
        return Err(PeerError::Unauthorized);
    }
    let identity = TransportIdentity::load(vault)?;
    let local = vault
        .load_identity(identity.peer_id().to_bytes())
        .map_err(|_| PeerError::Storage)?;
    let state = membership(&mut store, policy)?;
    local_current(&state, &local, false)?;
    let (auth, hello) = ClientSession::begin(
        local.public.clone(),
        identity.peer_id(),
        pin,
        Lane::Bulk,
        policy,
    )?;
    let mut auth = Some(auth);
    let mut swarm = identity.build_lane(Lane::Bulk)?;
    swarm.dial(address).map_err(|_| PeerError::Offline)?;
    let mut bulk = None;
    let mut expected = None;
    let mut connected = None;
    let mut cursor = 0;
    let mut chunk_size = 0;
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                if peer_id != pin.peer || connected.is_some() {
                    return Err(PeerError::Unauthorized);
                }
                connected = Some(connection_id);
                expected = Some(
                    swarm
                        .behaviour_mut()
                        .messages
                        .send_request(&pin.peer, hello.clone()),
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
                if Some(request_id) != expected.take()
                    || peer != pin.peer
                    || Some(connection_id) != connected
                {
                    return Err(PeerError::Session);
                }
                let state = membership(&mut store, policy)?;
                let now = Instant::now();
                let next = match response.body {
                    PeerBody::ServerHello { .. } => auth
                        .as_mut()
                        .ok_or(PeerError::Session)?
                        .server_hello(response, peer, connection_id, &state, &local.device_key)?,
                    PeerBody::Finished(_) => {
                        let mut active = auth.take().ok_or(PeerError::Session)?;
                        active.finished(response, peer, connection_id, &state)?;
                        chunk_size = active.binding()?.context.selected.chunk_bytes as usize;
                        let descriptor = BulkDescriptor {
                            transfer: fresh()?,
                            total: bytes.len() as u64,
                            chunks: bytes
                                .len()
                                .div_ceil(chunk_size)
                                .try_into()
                                .map_err(|_| PeerError::Limit)?,
                            digest: Sha256::digest(bytes).into(),
                        };
                        let mut b = BulkClient::new(active, &state)?;
                        let r = b.begin(descriptor, &state, now)?;
                        bulk = Some(b);
                        r
                    }
                    PeerBody::BulkChallenge { .. } => {
                        bulk.as_mut().ok_or(PeerError::Session)?.challenge(
                            response,
                            peer,
                            connection_id,
                            &state,
                            &local.device_key,
                            now,
                        )?
                    }
                    PeerBody::BulkReady { .. } => {
                        let b = bulk.as_mut().ok_or(PeerError::Session)?;
                        b.ready(response, peer, connection_id, &state, now)?;
                        let end = bytes.len().min(cursor + chunk_size);
                        let r = b.chunk(&bytes[cursor..end], &state, now)?;
                        cursor = end;
                        r
                    }
                    PeerBody::BulkProgress { .. } | PeerBody::BulkVerified { .. } => {
                        let b = bulk.as_mut().ok_or(PeerError::Session)?;
                        let result = b.progress(response, peer, connection_id, &state, now)?;
                        if matches!(result, BulkResult::Verified { .. }) {
                            return Ok(result);
                        }
                        let end = bytes.len().min(cursor + chunk_size);
                        let r = b.chunk(&bytes[cursor..end], &state, now)?;
                        cursor = end;
                        r
                    }
                    _ => return Err(PeerError::Unsupported),
                };
                expected = Some(swarm.behaviour_mut().messages.send_request(&pin.peer, next));
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
    }
}
