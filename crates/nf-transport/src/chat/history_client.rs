use super::{
    ChatPeerPin, Refusal, WireContext,
    history::{HistoryFrame, validate_page, validate_query},
    network::ChatBehaviourEvent,
    owner::local_current,
};
use crate::{PeerError, identity::TransportIdentity};
use futures::StreamExt;
use libp2p::{
    Multiaddr,
    multiaddr::Protocol,
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_identity::{
    model::{DeviceProof, MembershipState},
    private_storage::PrivateVault,
    signing::device_digest,
};
use nf_store::chat::{Author, ChatPolicy, HistoryPage, HistoryQuery, codec};
use std::time::Duration;
fn before_deadline(deadline: tokio::time::Instant) -> Result<(), PeerError> {
    if tokio::time::Instant::now() >= deadline {
        Err(PeerError::Offline)
    } else {
        Ok(())
    }
}
/// Read one permitted source-history entry, never modify an outbox or learn membership from a page.
/// Current LOCAL membership and a pinned actual Noise receiver remain trusted supervisor inputs.
pub async fn fetch_history(
    vault: &PrivateVault,
    current: &MembershipState,
    pin: &ChatPeerPin,
    address: Multiaddr,
    query: HistoryQuery,
) -> Result<HistoryPage, PeerError> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let policy = ChatPolicy {
        scope: current.scope,
    };
    let context = WireContext {
        policy_digest: codec::policy_digest(&policy),
        request: query.request,
    };
    validate_query(context, query)?;
    let address_bytes: &[u8] = address.as_ref();
    if address_bytes.len() > 128 {
        return Err(PeerError::Limit);
    }
    let fields: Vec<_> = address.iter().take(4).collect();
    if !matches!(fields.as_slice(), [Protocol::Ip4(ip), Protocol::Tcp(port), Protocol::P2p(expected)]
        if ip.is_loopback() && *port != 0 && *expected == pin.peer)
    {
        return Err(PeerError::Unauthorized);
    }
    let identity = TransportIdentity::load(vault);
    before_deadline(deadline)?;
    let identity = identity?;
    let local = vault.load_identity(identity.peer_id().to_bytes());
    before_deadline(deadline)?;
    let local = local.map_err(|_| PeerError::Storage)?;
    local_current(current, &local, identity.peer_id())?;
    let actor = Author {
        account: local.public.account,
        device: local.public.device,
    };
    let selected = ChatPeerPin::from_current(current, pin.receiver)?;
    if actor != query.reader
        || selected.peer != pin.peer
        || selected.key != pin.key
        || selected.scope != pin.scope
        || current.revision < pin.minimum_membership
    {
        return Err(PeerError::Unauthorized);
    }
    let swarm = identity.build_chat();
    before_deadline(deadline)?;
    let mut swarm = swarm?;
    swarm.dial(address).map_err(|_| PeerError::Offline)?;
    let mut connected = None;
    let mut pending = None;
    let mut awaiting_challenge = true;
    loop {
        let event = tokio::select! { biased;
            _ = tokio::time::sleep_until(deadline) => return Err(PeerError::Offline),
            event = swarm.select_next_some() => event,
        };
        before_deadline(deadline)?;
        match event {
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                if peer_id != pin.peer || connected.is_some() {
                    return Err(PeerError::Unauthorized);
                }
                connected = Some(connection_id);
                before_deadline(deadline)?;
                pending = Some(
                    swarm
                        .behaviour_mut()
                        .history
                        .send_request(&pin.peer, HistoryFrame::Challenge { context, query }),
                );
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Response {
                        request_id,
                        response,
                    },
            })) => {
                if peer != pin.peer
                    || connected != Some(connection_id)
                    || pending.take() != Some(request_id)
                    || response.context_query() != (context, query)
                {
                    return Err(PeerError::Session);
                }
                match response {
                    HistoryFrame::Issued { challenge, .. } if awaiting_challenge => {
                        if challenge.membership_revision != current.revision
                            || challenge.membership_revision < pin.minimum_membership
                        {
                            return Err(PeerError::Unauthorized);
                        }
                        let mut proof = DeviceProof {
                            scope: policy.scope,
                            account: actor.account,
                            device: actor.device,
                            frontier: challenge.membership_revision,
                            peer: local.public.peer.clone(),
                            challenge: challenge.challenge,
                            signature: [0; 64],
                        };
                        proof.signature = local
                            .device_key
                            .sign(&device_digest(&proof).map_err(|_| PeerError::Malformed)?);
                        before_deadline(deadline)?;
                        awaiting_challenge = false;
                        pending = Some(swarm.behaviour_mut().history.send_request(
                            &pin.peer,
                            HistoryFrame::Proof {
                                context,
                                query,
                                ticket: challenge.ticket,
                                proof: Box::new(proof),
                            },
                        ));
                    }
                    HistoryFrame::Page { page, .. } if !awaiting_challenge => {
                        validate_page(query, &page)?;
                        for entry in &page.entries {
                            let signed = &entry.signed;
                            if signed.message.scope != current.scope
                                || signed.message.channel != query.channel
                            {
                                return Err(PeerError::Policy);
                            }
                            let device = current
                                .devices
                                .get(&signed.message.author.device)
                                .ok_or(PeerError::Unauthorized)?;
                            if device.account != signed.message.author.account
                                || !current.accounts.contains_key(&device.account)
                            {
                                return Err(PeerError::Unauthorized);
                            }
                            // Historical signatures may survive sender revocation; no fresh posting authority is granted.
                            nf_contract::signatures::verify_digest(
                                &device.key,
                                &codec::message_digest(&signed.message)
                                    .map_err(|_| PeerError::Malformed)?,
                                &signed.signature,
                            )
                            .map_err(|_| PeerError::Unauthorized)?;
                        }
                        before_deadline(deadline)?;
                        return Ok(*page);
                    }
                    HistoryFrame::Refused { reason, .. } => {
                        return Err(match reason {
                            Refusal::Unsupported => PeerError::Unsupported,
                            Refusal::Unauthorized => PeerError::Unauthorized,
                            Refusal::Limit => PeerError::Limit,
                            Refusal::Replay => PeerError::Replay,
                            Refusal::Conflict => PeerError::Policy,
                            Refusal::Offline => PeerError::Offline,
                        });
                    }
                    _ => return Err(PeerError::Session),
                }
            }
            SwarmEvent::OutgoingConnectionError { .. }
            | SwarmEvent::ConnectionClosed { .. }
            | SwarmEvent::Behaviour(ChatBehaviourEvent::History(
                Event::OutboundFailure { .. } | Event::InboundFailure { .. },
            )) => {
                return Err(PeerError::Offline);
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message { .. }))
            | SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { .. })) => {
                return Err(PeerError::Session);
            }
            _ => {}
        }
    }
}
