use nf_contract::identity::*;
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::*,
    signing::{admission_proof, sign_invitation},
};
use nf_transport::{
    auth::{HandshakeContext, ServerPin},
    receipt::*,
    records::{Lane, PeerContext, PeerLimits},
};
pub struct Pair {
    pub server: PublicIdentity,
    pub client: PublicIdentity,
    pub server_key: SecretSeed,
    pub server_account_key: SecretSeed,
    pub client_key: SecretSeed,
    pub state: MembershipState,
    pub handshake: ReceiptHandshake,
    pub connection: libp2p::swarm::ConnectionId,
}
fn random<const N: usize>() -> [u8; N] {
    let mut b = [0; N];
    getrandom::fill(&mut b).unwrap();
    assert_ne!(b, [0; N]);
    b
}
impl Pair {
    pub fn new() -> Self {
        Self::with_roles(Roles::PLAYER)
    }
    pub fn with_roles(roles: Roles) -> Self {
        let sp = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let cp = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let (server, sa, server_key) = generate_identity(sp.to_bytes()).unwrap();
        let (client, ca, client_key) = generate_identity(cp.to_bytes()).unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes(random()),
            history: HistoryId::from_bytes(random()),
        };
        let founder = MembershipState::bootstrap(scope, &server).unwrap();
        let invitation = Invitation {
            scope,
            id: random(),
            issuer: server.account,
            recipient: client.clone(),
            roles,
            expires_at: 100,
            issued_revision: founder.revision,
            reusable: false,
        };
        let proof = admission_proof(&invitation, &ca, &client_key).unwrap();
        let state = founder
            .redeem(&sign_invitation(invitation, &sa).unwrap(), &proof, 1)
            .unwrap();
        let handshake = ReceiptHandshake::new(HandshakeContext {
            lane: Lane::Control,
            client_peer: cp,
            server_peer: sp,
            client_account: client.account,
            client_device: client.device,
            server_account: server.account,
            server_device: server.device,
            context: PeerContext {
                session: random(),
                scope,
                ruleset: random(),
                content: random(),
            },
            client_nonce: random(),
            server_nonce: random(),
            required: 1,
            optional: 0,
            server_available: 1,
            selected_caps: 1,
            offered: PeerLimits::default(),
            server_limits: PeerLimits::default(),
            selected: PeerLimits::default(),
        })
        .unwrap();
        Self {
            server,
            client,
            server_key,
            server_account_key: sa,
            client_key,
            state,
            handshake,
            connection: libp2p::swarm::ConnectionId::new_unchecked(1),
        }
    }
    pub fn sessions(&self) -> (ReceiptSession, ReceiptSession) {
        let c = self.handshake.context();
        let pin = ServerPin {
            peer: c.server_peer,
            account: c.server_account,
            device: c.server_device,
            minimum_membership: self.state.revision,
        };
        let make = |endpoint, local| {
            ReceiptSession::new(
                self.handshake.clone(),
                ReceiptSessionBinding {
                    endpoint,
                    local,
                    connection: self.connection,
                    server_pin: pin,
                    minimum_membership: self.state.revision,
                },
                &self.state,
            )
            .unwrap()
        };
        (
            make(ReceiptEndpoint::Client, self.client.clone()),
            make(ReceiptEndpoint::Server, self.server.clone()),
        )
    }
}
