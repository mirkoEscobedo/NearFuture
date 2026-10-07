#![allow(dead_code)] // Disposable support is shared by independent notification test targets.
pub mod lanes;
pub mod repo;
static FIXTURE_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn fixture_gate() -> std::sync::MutexGuard<'static, ()> {
    FIXTURE_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::*,
    signing::{admission_proof, sign_invitation},
};
use nf_kernel::{World, WorldSpec};
use nf_store::Store;
use nf_transport::{
    auth::ServerPin, notification::NotifyLimits, notification_effects::NotifyPolicy,
};
use std::path::PathBuf;
struct Scratch {
    original: PathBuf,
    workspace: PathBuf,
}
impl Scratch {
    fn new() -> Self {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap();
        let parent = workspace.join(".tmp/notification-effect-tests");
        std::fs::create_dir_all(&parent).unwrap();
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let path = parent.join(name);
        std::fs::create_dir(&path).unwrap();
        let original = path.canonicalize().unwrap();
        assert!(
            original.starts_with(&workspace)
                && original.parent().unwrap() == parent.canonicalize().unwrap()
        );
        Self {
            original,
            workspace,
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(target) = self.original.canonicalize() {
            assert!(
                target == self.original
                    && target.starts_with(self.workspace.join(".tmp/notification-effect-tests"))
            );
            std::fs::remove_dir_all(&target).unwrap();
        }
    }
}
/// Generated disposable keys stay in memory; only public signed membership is stored in SQLite.
pub struct Fixture {
    pub server: PublicIdentity,
    pub server_account: SecretSeed,
    pub server_device: SecretSeed,
    pub client: PublicIdentity,
    pub client_device: SecretSeed,
    pub server_peer: libp2p::PeerId,
    pub client_peer: libp2p::PeerId,
    pub store: Store,
    pub scope: Scope,
    _scratch: Scratch,
    _gate: std::sync::MutexGuard<'static, ()>,
}
impl Fixture {
    pub fn new(roles: Roles) -> Self {
        let gate = fixture_gate();
        let scratch = Scratch::new();
        let saves = scratch.original.join("saves");
        std::fs::create_dir(&saves).unwrap();
        let private = scratch.original.join("private");
        nf_identity::private_storage::PrivateVault::create(&private, &saves).unwrap();
        let server_peer = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let client_peer = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let (server, server_account, server_device) =
            generate_identity(server_peer.to_bytes()).unwrap();
        let (client, ca, client_device) = generate_identity(client_peer.to_bytes()).unwrap();
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
        let signed = sign_invitation(invite, &server_account).unwrap();
        let state = founder.redeem(&signed, &proof, 1).unwrap();
        let world = World::new(WorldSpec::empty(
            scope.universe,
            scope.history,
            [3; 32],
            [4; 32],
        ))
        .unwrap();
        let mut store = Store::create(private.join("policy.sqlite"), &world).unwrap();
        store.commit_membership(None, &founder).unwrap();
        store.commit_membership(Some(0), &state).unwrap();
        Self {
            server,
            server_account,
            server_device,
            client,
            client_device,
            server_peer,
            client_peer,
            store,
            scope,
            _scratch: scratch,
            _gate: gate,
        }
    }
    pub fn current(&mut self) -> MembershipState {
        self.store.load_membership(self.scope).unwrap().unwrap()
    }
    pub fn policy(&self) -> NotifyPolicy {
        NotifyPolicy {
            scope: self.scope,
            ruleset: [4; 32],
            content: [5; 32],
            limits: NotifyLimits::default(),
            minimum_membership: 1,
        }
    }
    pub fn pin(&self) -> ServerPin {
        ServerPin {
            peer: self.server_peer,
            account: self.server.account,
            device: self.server.device,
            minimum_membership: 1,
        }
    }
}
