use super::support::repo::RepoFixture;
use libp2p::swarm::ConnectionId;
use nf_transport::notification_effects::{
    NotifyClientHandshake, NotifyServerHandshake, NotifySession,
};
impl RepoFixture {
    pub fn sessions(&self) -> (NotifySession, NotifySession) {
        let connection = ConnectionId::new_unchecked(7);
        let (mut client, hello) = NotifyClientHandshake::begin(
            self.client.public.clone(),
            self.client_peer,
            self.original.source(),
            self.policy(),
        )
        .unwrap();
        let (mut server, reply) = NotifyServerHandshake::begin(
            hello,
            self.client_peer,
            connection,
            self.server.public.clone(),
            self.server_peer,
            self.policy(),
            &self.state,
            &self.server.device_key,
        )
        .unwrap();
        let proof = client
            .server_hello(
                reply,
                self.server_peer,
                connection,
                &self.state,
                &self.client.device_key,
            )
            .unwrap();
        let (server, finished) = server
            .client_proof(
                proof,
                self.client_peer,
                connection,
                &self.state,
                &self.server.device_key,
            )
            .unwrap();
        let client = client
            .finished(finished, self.server_peer, connection, &self.state)
            .unwrap();
        (server, client)
    }
}
