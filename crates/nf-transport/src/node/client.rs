use super::{local_current, membership};
use crate::{
    PeerError,
    auth::ServerPin,
    identity::TransportIdentity,
    network::PeerBehaviourEvent,
    query::{QueryClient, QueryResult},
    reachability::{
        Diagnostic, EndpointRefusal, Failure, Observation, StatusReport, classify_dial_error,
        validate_explicit_bootstrap,
    },
    records::{Lane, PeerBody},
    session::{ClientSession, SessionPolicy},
};
use futures::StreamExt;
use libp2p::{
    Multiaddr,
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_contract::identity::RequestId;
use nf_identity::private_storage::PrivateVault;
use nf_store::Store;
use std::time::{Duration, Instant};
/// Explicit trusted IPv4 loopback address/pin; this partial transport installs no discovery or remote membership state.
pub async fn request_status(
    store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    address: Multiaddr,
    request: RequestId,
) -> Result<QueryResult, PeerError> {
    request_status_diagnosed(store, vault, policy, pin, Some(address), request)
        .await
        .result
}

/// Runs the same bounded, authenticated query and reports only closed diagnostic codes.
/// A connection observation alone does not grant authority or prove NAT reachability.
pub async fn request_status_diagnosed(
    store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    address: Option<Multiaddr>,
    request: RequestId,
) -> StatusReport {
    let mut diagnostic = Diagnostic::default();
    let result = match tokio::time::timeout(
        Duration::from_secs(5),
        request_inner(store, vault, policy, pin, address, request, &mut diagnostic),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            diagnostic.failure = Some(Failure::Deadline);
            Err(PeerError::Offline)
        }
    };
    if let Err(error) = &result
        && diagnostic.failure.is_none()
    {
        diagnostic.failure = Some(Failure::Operation(*error));
    }
    StatusReport { result, diagnostic }
}
async fn request_inner(
    mut store: Store,
    vault: &PrivateVault,
    policy: SessionPolicy,
    pin: ServerPin,
    address: Option<Multiaddr>,
    request: RequestId,
    diagnostic: &mut Diagnostic,
) -> Result<QueryResult, PeerError> {
    if let Err(refusal) = validate_explicit_bootstrap(pin.peer, address.as_ref()) {
        diagnostic.failure = Some(Failure::Endpoint(refusal));
        return Err(if refusal == EndpointRefusal::NoConfiguredPeer {
            PeerError::Offline
        } else {
            PeerError::Unauthorized
        });
    }
    let address = address.ok_or(PeerError::Offline)?;
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
        Lane::Control,
        policy,
    )?;
    let mut swarm = identity.build_lane(Lane::Control)?;
    if let Err(error) = swarm.dial(address) {
        diagnostic.failure = Some(Failure::Dial(classify_dial_error(&error)));
        return Err(PeerError::Offline);
    }
    diagnostic.observation = Observation::DialAttempted;
    let mut auth = Some(auth);
    let mut store = Some(store);
    let mut query = None;
    let mut expected = None;
    let mut connected = None;
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
                diagnostic.observation = Observation::DirectConnectionObserved;
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
                let next = match response.body {
                    PeerBody::ServerHello { .. } => {
                        let state = membership(store.as_mut().ok_or(PeerError::Session)?, policy)?;
                        auth.as_mut().ok_or(PeerError::Session)?.server_hello(
                            response,
                            peer,
                            connection_id,
                            &state,
                            &local.device_key,
                        )?
                    }
                    PeerBody::Finished(_) => {
                        let state = membership(store.as_mut().ok_or(PeerError::Session)?, policy)?;
                        let mut active = auth.take().ok_or(PeerError::Session)?;
                        active.finished(response, peer, connection_id, &state)?;
                        let mut q =
                            QueryClient::new(active, store.take().ok_or(PeerError::Session)?)?;
                        let begin = q.begin(request, Instant::now())?;
                        query = Some(q);
                        begin
                    }
                    PeerBody::QueryChallenge { .. } => {
                        query.as_mut().ok_or(PeerError::Session)?.challenge(
                            response,
                            peer,
                            connection_id,
                            &local.device_key,
                            Instant::now(),
                        )?
                    }
                    PeerBody::RetainedStatus { .. } | PeerBody::Unsupported { .. } => {
                        let result = query.as_mut().ok_or(PeerError::Session)?.reply(
                            response,
                            peer,
                            connection_id,
                            Instant::now(),
                        );
                        if result.is_ok() {
                            diagnostic.observation = Observation::ReplyValidated;
                        }
                        return result;
                    }
                    _ => return Err(PeerError::Unsupported),
                };
                expected = Some(swarm.behaviour_mut().messages.send_request(&pin.peer, next));
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                diagnostic.failure = Some(Failure::Dial(classify_dial_error(&error)));
                return Err(PeerError::Offline);
            }
            SwarmEvent::ConnectionClosed { .. } => {
                diagnostic.failure = Some(Failure::ConnectionClosed);
                return Err(PeerError::Offline);
            }
            SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(
                Event::OutboundFailure { .. }
                | Event::InboundFailure { .. }
                | Event::Message { .. },
            )) => {
                diagnostic.failure = Some(Failure::RequestResponse);
                return Err(PeerError::Offline);
            }
            _ => {}
        }
    }
}
