#![allow(dead_code)] // Shared private/durable fixture is compiled by separate owner tests.
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    model::*,
    private_storage::{LocalIdentity, PrivateVault},
    signing::{admission_proof, sign_invitation},
};
use nf_kernel::{World, WorldSpec};
use nf_store::Store;
use nf_transport::identity::TransportIdentity;
pub struct OwnedCommunity {
    pub server_vault: PrivateVault,
    pub client_vault: PrivateVault,
    pub server: LocalIdentity,
    pub client: LocalIdentity,
    pub state: MembershipState,
    pub server_store: Store,
    pub client_store: Store,
}
impl OwnedCommunity {
    pub fn new(root: &std::path::Path) -> Self {
        Self::with_role(root, Roles::PLAYER)
    }
    pub fn with_role(root: &std::path::Path, roles: Roles) -> Self {
        let saves = root.join("saves");
        std::fs::create_dir(&saves).unwrap();
        let server_vault = PrivateVault::create(&root.join("server-private"), &saves).unwrap();
        let client_vault = PrivateVault::create(&root.join("client-private"), &saves).unwrap();
        let st = TransportIdentity::create(&server_vault).unwrap();
        let ct = TransportIdentity::create(&client_vault).unwrap();
        let server = server_vault
            .create_identity(st.peer_id().to_bytes())
            .unwrap();
        let client = client_vault
            .create_identity(ct.peer_id().to_bytes())
            .unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let founder = MembershipState::bootstrap(scope, &server.public).unwrap();
        let invite = Invitation {
            scope,
            id: [3; 16],
            issuer: server.public.account,
            recipient: client.public.clone(),
            roles,
            expires_at: 100,
            issued_revision: 0,
            reusable: false,
        };
        let proof = admission_proof(&invite, &client.account_key, &client.device_key).unwrap();
        let signed = sign_invitation(invite, &server.account_key).unwrap();
        let state = founder.redeem(&signed, &proof, 1).unwrap();
        let world = World::new(WorldSpec::empty(
            scope.universe,
            scope.history,
            [3; 32],
            [4; 32],
        ))
        .unwrap();
        let mut server_store = Store::create(root.join("server.sqlite"), &world).unwrap();
        let mut client_store = Store::create(root.join("client.sqlite"), &world).unwrap();
        for s in [&mut server_store, &mut client_store] {
            s.commit_membership(None, &founder).unwrap();
            s.commit_membership(Some(0), &state).unwrap();
        }
        Self {
            server_vault,
            client_vault,
            server,
            client,
            state,
            server_store,
            client_store,
        }
    }
}
