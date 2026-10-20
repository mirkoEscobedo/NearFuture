//! Test-owned Noise orchestration only; expected quota/status values stay in the scenario file.
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    request_response::{Event, Message},
    swarm::{ConnectionId, SwarmEvent},
};
use nf_identity::private_storage::PrivateVault;
use nf_transport::{
    chat::{
        ChatFrame, ChatPeerServer, ChatPeerServerEvent,
        network::{ChatBehaviour, ChatBehaviourEvent},
    },
    identity::TransportIdentity,
};
pub(super) struct Raw {
    swarm: Swarm<ChatBehaviour>,
    peer: PeerId,
    pub(super) ids: Option<(ConnectionId, ConnectionId)>,
}
impl Raw {
    pub(super) fn new(vault: &PrivateVault) -> Self {
        let identity = TransportIdentity::load(vault).unwrap();
        Self {
            swarm: identity.build_chat().unwrap(),
            peer: identity.peer_id(),
            ids: None,
        }
    }
    pub(super) async fn connect(
        &mut self,
        server: &mut ChatPeerServer,
        address: Multiaddr,
        expected: PeerId,
    ) {
        self.swarm.dial(address).unwrap();
        let (mut client, mut owner) = (None, None);
        while client.is_none() || owner.is_none() {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, connection_id, .. } => { assert_eq!(peer_id, expected); client = Some(connection_id); }
                    SwarmEvent::OutgoingConnectionError { error, .. } => panic!("genuine Noise connect failed: {error}"),
                    _ => {}
                },
                event = server.next() => if let ChatPeerServerEvent::Connected { peer, connection } = event.unwrap() {
                    assert_eq!(peer, self.peer); owner = Some(connection);
                }
            }
        }
        self.ids = Some((client.unwrap(), owner.unwrap()));
    }
    pub(super) async fn post(
        &mut self,
        server: &mut ChatPeerServer,
        expected: PeerId,
        frame: ChatFrame,
    ) -> ChatFrame {
        let request = self
            .swarm
            .behaviour_mut()
            .messages
            .send_request(&expected, frame);
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { peer, connection_id, message: Message::Response { request_id, response } })) => {
                        assert_eq!(peer, expected); assert_eq!(connection_id, self.ids.unwrap().0); assert_eq!(request_id, request); return response;
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::OutboundFailure { error, .. })) => panic!("post exchange failed: {error}"),
                    _ => {}
                },
                event = server.next() => { event.unwrap(); }
            }
        }
    }
    pub(super) async fn disconnect(&mut self, server: &mut ChatPeerServer, expected: PeerId) {
        let ids = self.ids.take().unwrap();
        assert!(self.swarm.close_connection(ids.0));
        let (mut client, mut owner) = (false, false);
        while !client || !owner {
            tokio::select! {
                event = self.swarm.select_next_some() => if let SwarmEvent::ConnectionClosed { peer_id, connection_id, .. } = event {
                    assert_eq!(peer_id, expected); assert_eq!(connection_id, ids.0); client = true;
                },
                event = server.next() => if let ChatPeerServerEvent::Disconnected { peer, connection } = event.unwrap() {
                    assert_eq!(peer, self.peer); assert_eq!(connection, ids.1); owner = true;
                }
            }
        }
    }
}

// Additional physical History/refusal routes; qualified first orchestration above is unchanged.
impl Raw {
    pub(super) async fn history(
        &mut self,
        server: &mut ChatPeerServer,
        expected: PeerId,
        frame: nf_transport::chat::history::HistoryFrame,
    ) -> nf_transport::chat::history::HistoryFrame {
        let request = self
            .swarm
            .behaviour_mut()
            .history
            .send_request(&expected, frame);
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message { peer, connection_id, message: Message::Response { request_id, response } })) => {
                        assert_eq!(peer, expected); assert_eq!(connection_id, self.ids.unwrap().0); assert_eq!(request_id, request); return response;
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::OutboundFailure { error, .. })) => panic!("history exchange failed: {error}"),
                    _ => {}
                },
                event = server.next() => { event.unwrap(); }
            }
        }
    }
    pub(super) async fn refused_and_closed(
        &mut self,
        server: &mut ChatPeerServer,
        expected: PeerId,
        frame: ChatFrame,
        refusal: ChatFrame,
    ) {
        let ids = self.ids.take().unwrap();
        let request = self
            .swarm
            .behaviour_mut()
            .messages
            .send_request(&expected, frame);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let (mut response_seen, mut client_closed, mut owner_closed) = (false, false, false);
            while !response_seen || !client_closed || !owner_closed {
                tokio::select! {
                    event = self.swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { peer, connection_id, message: Message::Response { request_id, response } })) => {
                            assert_eq!(peer, expected); assert_eq!(connection_id, ids.0); assert_eq!(request_id, request); assert_eq!(response, refusal); response_seen = true;
                        }
                        SwarmEvent::ConnectionClosed { peer_id, connection_id, .. } => { assert_eq!(peer_id, expected); assert_eq!(connection_id, ids.0); client_closed = true; }
                        SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::OutboundFailure { error, .. })) => panic!("bounded refusal was not returned: {error}"),
                        _ => {}
                    },
                    event = server.next() => if let ChatPeerServerEvent::Disconnected { peer, connection } = event.unwrap() {
                        assert_eq!(peer, self.peer); assert_eq!(connection, ids.1); owner_closed = true;
                    }
                }
            }
        }).await.expect("denied physical lane must close after its bounded refusal, before the old ten-second idle timeout");
    }
}
