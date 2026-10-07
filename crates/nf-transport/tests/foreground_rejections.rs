#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;
use futures::StreamExt;
use libp2p::{
    Swarm,
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_identity::{
    model::{DeviceProof, Roles},
    signing::device_digest,
};
use nf_transport::{
    auth::HandshakeContext,
    identity::TransportIdentity,
    network::{PeerBehaviour, PeerBehaviourEvent},
    node::{PeerServer, PeerServerEvent},
    records::{Lane, PeerBody, PeerContext, PeerLimits, PeerRecord},
    session::SessionPolicy,
};
async fn response(
    server: &mut PeerServer,
    client: &mut Swarm<PeerBehaviour>,
) -> Option<PeerRecord> {
    loop {
        tokio::select! {event=client.select_next_some()=>match event{SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message{message:Message::Response{response,..},..}))=>return Some(response),SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::OutboundFailure{..}))|SwarmEvent::ConnectionClosed{..}|SwarmEvent::OutgoingConnectionError{..}=>return None,_=>{}},event=server.next()=>{event.unwrap();}}
    }
}
#[tokio::test(flavor = "current_thread")]
async fn actual_noise_server_refuses_foreign_scope_history_and_ruleset_without_authoritative_result()
 {
    for field in 0..3 {
        tokio::time::timeout(std::time::Duration::from_secs(8), async {
            let tmp = scratch::Scratch::new();
            let c = owned_community::OwnedCommunity::new(&tmp.0);
            let policy = SessionPolicy {
                scope: c.state.scope,
                ruleset: [4; 32],
                content: [5; 32],
                limits: PeerLimits::default(),
                minimum_membership: 1,
            };
            let mut server = PeerServer::new(c.server_store, &c.server_vault, policy).unwrap();
            let (address, sp) = loop {
                if let PeerServerEvent::Ready { control, peer, .. } = server.next().await.unwrap() {
                    break (control, peer);
                }
            };
            let identity = TransportIdentity::load(&c.client_vault).unwrap();
            let mut client = identity.build_lane(Lane::Control).unwrap();
            client.dial(address).unwrap();
            let mut context = PeerContext {
                session: [0; 16],
                scope: c.state.scope,
                ruleset: [4; 32],
                content: [5; 32],
            };
            match field {
                0 => {
                    context.scope.universe = nf_contract::identity::UniverseId::from_bytes([99; 16])
                }
                1 => context.scope.history = nf_contract::identity::HistoryId::from_bytes([99; 16]),
                _ => context.ruleset = [99; 32],
            };
            client.behaviour_mut().messages.send_request(
                &sp,
                PeerRecord {
                    context,
                    body: PeerBody::Hello {
                        account: c.client.public.account,
                        device: c.client.public.device,
                        nonce: [7; 32],
                        required: 1,
                        optional: 2,
                        offered: PeerLimits::default(),
                    },
                },
            );
            assert!(response(&mut server, &mut client).await.is_none());
        })
        .await
        .expect("owned foreign context rejection deadline");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn actual_noise_peer_with_valid_worker_signature_cannot_promote_itself_to_player() {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        let tmp = scratch::Scratch::new();
        let c = owned_community::OwnedCommunity::with_role(&tmp.0, Roles::WORKER);
        let policy = SessionPolicy {
            scope: c.state.scope,
            ruleset: [4; 32],
            content: [5; 32],
            limits: PeerLimits::default(),
            minimum_membership: 1,
        };
        let mut server = PeerServer::new(c.server_store, &c.server_vault, policy).unwrap();
        let (address, sp) = loop {
            if let PeerServerEvent::Ready { control, peer, .. } = server.next().await.unwrap() {
                break (control, peer);
            }
        };
        let identity = TransportIdentity::load(&c.client_vault).unwrap();
        let cp = identity.peer_id();
        let mut client = identity.build_lane(Lane::Control).unwrap();
        client.dial(address).unwrap();
        client.behaviour_mut().messages.send_request(
            &sp,
            PeerRecord {
                context: PeerContext {
                    session: [0; 16],
                    scope: c.state.scope,
                    ruleset: [4; 32],
                    content: [5; 32],
                },
                body: PeerBody::Hello {
                    account: c.client.public.account,
                    device: c.client.public.device,
                    nonce: [7; 32],
                    required: 1,
                    optional: 2,
                    offered: PeerLimits::default(),
                },
            },
        );
        let reply = response(&mut server, &mut client).await.unwrap();
        let PeerBody::ServerHello {
            nonce,
            available,
            selected_caps,
            server_limits,
            selected,
            proof: server_proof,
        } = reply.body
        else {
            panic!()
        };
        let h = HandshakeContext {
            lane: Lane::Control,
            client_peer: cp,
            server_peer: sp,
            client_account: c.client.public.account,
            client_device: c.client.public.device,
            server_account: server_proof.account,
            server_device: server_proof.device,
            context: reply.context,
            client_nonce: [7; 32],
            server_nonce: nonce,
            required: 1,
            optional: 2,
            server_available: available,
            selected_caps,
            offered: PeerLimits::default(),
            server_limits,
            selected,
        };
        let mut proof = DeviceProof {
            scope: c.state.scope,
            account: c.client.public.account,
            device: c.client.public.device,
            frontier: c.state.revision,
            peer: cp.to_bytes(),
            challenge: h.challenge(2, c.state.revision).unwrap(),
            signature: [0; 64],
        };
        proof.signature = c.client.device_key.sign(&device_digest(&proof).unwrap());
        client.behaviour_mut().messages.send_request(
            &sp,
            PeerRecord {
                context: reply.context,
                body: PeerBody::ClientProof(proof),
            },
        );
        assert!(response(&mut server, &mut client).await.is_none());
    })
    .await
    .expect("owned valid-worker rejection deadline");
}
