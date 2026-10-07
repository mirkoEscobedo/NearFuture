#![allow(dead_code)] // Shared signed scenario is compiled by separate behavior tests.
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::*,
    signing::{admission_proof, sign_invitation},
};
pub struct Community {
    pub server: PublicIdentity,
    pub server_account: SecretSeed,
    pub server_device: SecretSeed,
    pub client: PublicIdentity,
    pub client_device: SecretSeed,
    pub state: MembershipState,
    pub server_noise: libp2p::identity::Keypair,
    pub client_noise: libp2p::identity::Keypair,
}
impl Community {
    pub fn new(roles: Roles) -> Self {
        let server_noise = libp2p::identity::Keypair::generate_ed25519();
        let client_noise = libp2p::identity::Keypair::generate_ed25519();
        let (server, sa, server_device) =
            generate_identity(server_noise.public().to_peer_id().to_bytes()).unwrap();
        let (client, ca, client_device) =
            generate_identity(client_noise.public().to_peer_id().to_bytes()).unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let founder = MembershipState::bootstrap(scope, &server).unwrap();
        let invite = Invitation {
            scope,
            id: [3; 16],
            issuer: server.account,
            recipient: client.clone(),
            roles,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let proof = admission_proof(&invite, &ca, &client_device).unwrap();
        let signed = sign_invitation(invite, &sa).unwrap();
        let state = founder.redeem(&signed, &proof, 1).unwrap();
        Self {
            server,
            server_account: sa,
            server_device,
            client,
            client_device,
            state,
            server_noise,
            client_noise,
        }
    }
}
