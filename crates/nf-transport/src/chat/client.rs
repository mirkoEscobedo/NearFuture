use super::{
    ChatFrame, ChatPeerPin, Refusal, WireContext,
    network::ChatBehaviourEvent,
    owner::{local_current, store_error},
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
use nf_store::chat::{
    Author, ChatPolicy, codec as store_codec,
    outbox::{ClientOutbox, OutgoingEntry},
};
use sha2::{Digest, Sha256};
use std::time::Duration;
const DELIVERY_AGE: Duration = Duration::from_secs(5);
const MAX_ADDRESS_BYTES: usize = 128;
fn before_deadline(deadline: tokio::time::Instant) -> Result<(), PeerError> {
    if tokio::time::Instant::now() >= deadline {
        Err(PeerError::Offline)
    } else {
        Ok(())
    }
}
fn refused(reason: Refusal) -> PeerError {
    match reason {
        Refusal::Unsupported => PeerError::Unsupported,
        Refusal::Unauthorized => PeerError::Unauthorized,
        Refusal::Limit => PeerError::Limit,
        Refusal::Replay => PeerError::Replay,
        Refusal::Conflict => PeerError::Policy,
        Refusal::Offline => PeerError::Offline,
    }
}
fn peer_digest(peer: &[u8]) -> [u8; 32] {
    let mut bytes = b"NF-CHAT-RECEIVER-PEER-1\0".to_vec();
    bytes.extend_from_slice(&(peer.len() as u16).to_be_bytes());
    bytes.extend_from_slice(peer);
    Sha256::digest(bytes).into()
}
/// Foreground delivery of one retained original. Failure leaves the borrowed outbox owner available.
/// `current` and `pin` are trusted LOCAL selections; neither is enrolled from wire fields.
pub async fn deliver_pending(
    outbox: &mut ClientOutbox,
    vault: &PrivateVault,
    current: &MembershipState,
    pin: &ChatPeerPin,
    address: Multiaddr,
    message: [u8; 16],
) -> Result<OutgoingEntry, PeerError> {
    let deadline = tokio::time::Instant::now() + DELIVERY_AGE;
    let address_bytes: &[u8] = address.as_ref();
    if address_bytes.len() > MAX_ADDRESS_BYTES {
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
    let peer = identity.peer_id();
    let local = vault.load_identity(peer.to_bytes());
    before_deadline(deadline)?;
    let local = local.map_err(|_| PeerError::Storage)?;
    local_current(current, &local, peer)?;
    let selected = ChatPeerPin::from_current(current, pin.receiver)?;
    if selected.peer != pin.peer
        || selected.key != pin.key
        || selected.scope != pin.scope
        || current.revision < pin.minimum_membership
    {
        return Err(PeerError::Unauthorized);
    }
    before_deadline(deadline)?;
    let original = outbox.entry(message);
    before_deadline(deadline)?;
    let original = original.map_err(store_error)?.ok_or(PeerError::Malformed)?;
    let actor = Author {
        account: local.public.account,
        device: local.public.device,
    };
    if original.signed.message.author != actor
        || original.signed.message.scope != current.scope
        || pin.scope != current.scope
    {
        return Err(PeerError::Policy);
    }
    let policy = ChatPolicy {
        scope: original.signed.message.scope,
    };
    let context = WireContext {
        policy_digest: store_codec::policy_digest(&policy),
        request: original.original_request,
    };
    let swarm = identity.build_chat();
    before_deadline(deadline)?;
    let mut swarm = swarm?;
    swarm.dial(address).map_err(|_| PeerError::Offline)?;
    let mut connected = None;
    let mut pending = None;
    let mut awaiting_challenge = true;
    loop {
        let event = tokio::select! {
            biased;
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
                before_deadline(deadline)?;
                connected = Some(connection_id);
                pending = Some(swarm.behaviour_mut().messages.send_request(
                    &pin.peer,
                    ChatFrame::PostChallenge {
                        context,
                        signed: Box::new(original.signed.clone()),
                    },
                ));
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message {
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
                {
                    return Err(PeerError::Session);
                }
                // Correlation and actual Noise connection are checked before any receipt acknowledgment.
                let received_context = match &response {
                    ChatFrame::Issued { context, .. }
                    | ChatFrame::Delivered { context, .. }
                    | ChatFrame::Refused { context, .. } => *context,
                    _ => return Err(PeerError::Malformed),
                };
                if received_context != context {
                    return Err(PeerError::Session);
                }
                match response {
                    ChatFrame::Issued { challenge, .. } if awaiting_challenge => {
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
                        pending = Some(swarm.behaviour_mut().messages.send_request(
                            &pin.peer,
                            ChatFrame::PostProof {
                                context,
                                signed: Box::new(original.signed.clone()),
                                ticket: challenge.ticket,
                                proof: Box::new(proof),
                            },
                        ));
                    }
                    ChatFrame::Delivered { signed, .. } if !awaiting_challenge => {
                        if signed.receipt.receiver != pin.receiver
                            || signed.receipt.scope != pin.scope
                            || signed.receipt.receiver_peer_digest != peer_digest(&peer.to_bytes())
                            || signed.receipt.original.original_request != original.original_request
                        {
                            return Err(PeerError::Unauthorized);
                        }
                        // The store verifies its own immutable retained receiver signature/key and every original field.
                        before_deadline(deadline)?;
                        // This existing synchronous commit cannot be preempted; it may finish after the deadline.
                        return outbox.acknowledge(&signed).map_err(store_error);
                    }
                    ChatFrame::Refused { reason, .. } => return Err(refused(reason)),
                    _ => return Err(PeerError::Session),
                }
            }
            SwarmEvent::OutgoingConnectionError { .. }
            | SwarmEvent::ConnectionClosed { .. }
            | SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(
                Event::OutboundFailure { .. } | Event::InboundFailure { .. },
            )) => return Err(PeerError::Offline),
            SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { .. })) => {
                return Err(PeerError::Session);
            }
            _ => {}
        }
    }
}
