mod common;
use common::OwnedOutput;
use nf_contract::identity::*;
use nf_identity::{
    model::{MembershipRepository, MembershipState, Scope},
    private_storage::PrivateVault,
};
use nf_ipc::DiscoveryRecord;
use nf_kernel::{AuthorityContext, Command, Intent, World, WorldSpec};
use nf_store::{PrincipalDevice, Store};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command as ProcessCommand, Stdio},
    time::{Duration, Instant},
};
struct Owned {
    root: PathBuf,
    child: Option<Child>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn command() -> ProcessCommand {
    let mut c = ProcessCommand::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.stdin(Stdio::null());
    c
}
#[test]
fn foreground_store_query_reads_retained_pending_after_database_restart() {
    let root = std::env::temp_dir().join(format!("nf-ipc-store-process-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut owned = Owned {
        root: root.clone(),
        child: None,
    };
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault_path = root.join("vault");
    let vault = PrivateVault::create(&vault_path, &saves).unwrap();
    let identity = vault.create_identity(vec![4, 5]).unwrap();
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let world = World::new(WorldSpec::empty(
        scope.universe,
        scope.history,
        [3; 32],
        [4; 32],
    ))
    .unwrap();
    let database = root.join("history.sqlite");
    let mut store = Store::create(&database, &world).unwrap();
    store
        .commit_membership(
            None,
            &MembershipState::bootstrap(scope, &identity.public).unwrap(),
        )
        .unwrap();
    let intent = Intent {
        request: RequestId::from_bytes([8; 16]),
        operation: OperationId::from_bytes([8; 16]),
        job: JobId::from_bytes([8; 16]),
        actor: identity.public.account,
        universe: scope.universe,
        history: scope.history,
        provider: ProviderId::from_bytes([9; 16]),
        expected: Default::default(),
        command: Command::AdjustMarket {
            market: EntityId::from_bytes([7; 16]),
            delta: 1,
        },
    };
    let frontier = nf_kernel::admit(
        &world,
        vec![intent],
        AuthorityContext {
            term: AuthorityTerm(1),
            session: RuntimeSession(9),
        },
    )
    .unwrap();
    store
        .prepare(
            &frontier,
            &[PrincipalDevice {
                request: RequestId::from_bytes([8; 16]),
                device: identity.public.device,
            }],
            &[],
        )
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    drop(identity);
    let mut node = command();
    node.arg("serve-store")
        .arg(&vault_path)
        .arg(&saves)
        .arg("durable-query")
        .arg(&database)
        .args([
            "01".repeat(16),
            "02".repeat(16),
            "04".repeat(32),
            "04".repeat(32),
            "0405".into(),
            known.event_sequence.0.to_string(),
            known.store_revision.to_string(),
            known.membership_revision.unwrap().to_string(),
            "4000".into(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    owned.child = Some(node.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        assert!(
            owned.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "durable query node exited before publication"
        );
        if DiscoveryRecord::read(&vault, "durable-query").is_ok() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let reply = command()
        .arg("query")
        .arg(&vault_path)
        .arg(&saves)
        .arg("durable-query")
        .arg("08".repeat(16))
        .bounded_output();
    assert!(reply.status.success());
    assert_eq!(
        String::from_utf8(reply.stdout).unwrap().trim(),
        format!(
            "RETAINED phase {}",
            nf_wire::generated::OperationPhase::Pending as i32
        )
    );
    assert!(reply.stderr.is_empty());
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = owned.child.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    owned.child.take();
    assert!(!vault_path.join("blob-durable-query").exists());
    // Read-only query node never commits or resolves the retained operation.
    let reopened = Store::open_existing(database, known).unwrap();
    assert!(reopened.pending().is_some());
    drop(reopened);
}
