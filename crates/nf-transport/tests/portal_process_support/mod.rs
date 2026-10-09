pub mod config;
pub mod process;
pub mod repo;
use libp2p::PeerId;
use nf_contract::identity::*;
use nf_store::{BoundRequestStatus, KnownFrontiers, Store};
use nf_transport::{receipt::OriginalReceipt, receipt_effects::book::BookAnchor};
use std::path::PathBuf;
static FIXTURE_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn fixture_gate() -> std::sync::MutexGuard<'static, ()> {
    FIXTURE_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
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
        let parent = workspace.join(".tmp/portal-process-tests");
        std::fs::create_dir_all(&parent).unwrap();
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).unwrap();
        let name: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
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
                    && target.starts_with(self.workspace.join(".tmp/portal-process-tests"))
            );
            let _ = std::fs::remove_dir_all(target);
        }
    }
}
pub struct ProcessFixture {
    pub root: PathBuf,
    pub server_account: AccountId,
    pub server_device: DeviceId,
    pub client_account: AccountId,
    pub client_device: DeviceId,
    pub server_peer: PeerId,
    pub client_peer: PeerId,
    pub server_known: KnownFrontiers,
    pub client_known: KnownFrontiers,
    pub original: OriginalReceipt,
    pub anchor: BookAnchor,
    baseline_world: [u8; 32],
    baseline_pending: [u8; 32],
    baseline_binding: BoundRequestStatus,
    _scratch: Scratch,
    _gate: std::sync::MutexGuard<'static, ()>,
}
impl ProcessFixture {
    pub fn assert_original_pending_unchanged(&self) {
        let store = Store::open_existing(self.root.join("server/policy.sqlite"), self.server_known)
            .unwrap();
        assert_eq!(store.known_frontiers().unwrap(), self.server_known);
        assert_eq!(store.revision(), 1);
        assert_eq!(
            nf_kernel::state_hash(store.world()).unwrap(),
            self.baseline_world
        );
        assert_eq!(store.pending().unwrap().input_hash(), self.baseline_pending);
        assert_eq!(
            store
                .query_bound(
                    self.original.request(),
                    self.client_account,
                    self.client_device,
                    self.server_known.scope
                )
                .unwrap()
                .unwrap(),
            self.baseline_binding
        );
        assert_eq!(store.outbox().count(), 0);
        assert_eq!(store.reservations().count(), 0);
    }
}
