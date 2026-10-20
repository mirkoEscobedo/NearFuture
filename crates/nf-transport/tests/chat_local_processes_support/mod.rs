use nf_contract::identity::RequestId;
use nf_identity::private_storage::PrivateVault;
use nf_store::chat::{
    Author, Channel, ChatPolicy, ChatStore, HistoryQuery, KnownChatFrontiers,
    outbox::{ClientOutbox, OutboxProfile, OutgoingState},
};
use nf_transport::{
    chat::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, deliver_pending, fetch_history},
    identity::TransportIdentity,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child as Process, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Child {
    process: Process,
    stdout: PathBuf,
    stderr: PathBuf,
}
impl Child {
    pub fn spawn(
        root: &Path,
        evidence: &Path,
        mode: &str,
        who: &str,
        phase: &str,
        address: &Path,
        stop: &Path,
    ) -> Self {
        let stdout = evidence.join(format!("{mode}-{who}-{phase}.stdout.original"));
        let stderr = evidence.join(format!("{mode}-{who}-{phase}.stderr.original"));
        let open = |path: &Path| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap()
        };
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "owned_chat_process_worker",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("NF_CHAT_LOCAL_PROCESS_ROOT", root)
            .env("NF_CHAT_LOCAL_PROCESS_MODE", mode)
            .env("NF_CHAT_LOCAL_PROCESS_WHO", who)
            .env("NF_CHAT_LOCAL_PROCESS_ADDRESS", address)
            .env("NF_CHAT_LOCAL_PROCESS_STOP", stop)
            .stdin(Stdio::null())
            .stdout(Stdio::from(open(&stdout)))
            .stderr(Stdio::from(open(&stderr)));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        Self {
            process: command.spawn().unwrap(),
            stdout,
            stderr,
        }
    }
    pub fn ready(&mut self, path: &Path, budget: Duration) {
        let until = Instant::now() + budget;
        loop {
            if path.is_file() {
                return;
            }
            assert!(
                self.process.try_wait().unwrap().is_none(),
                "sidecar exited before readiness: {}",
                fs::read_to_string(&self.stderr).unwrap()
            );
            assert!(
                Instant::now() < until,
                "owned Chat sidecar readiness deadline"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn wait(&mut self, budget: Duration) {
        let until = Instant::now() + budget;
        loop {
            if let Some(status) = self.process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "owned Chat child failed: {} {}",
                    self.stdout(),
                    fs::read_to_string(&self.stderr).unwrap()
                );
                assert!(fs::read(&self.stderr).unwrap().is_empty());
                return;
            }
            assert!(Instant::now() < until, "owned Chat child deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn stdout(&self) -> String {
        let bytes = fs::read(&self.stdout).unwrap();
        assert!(bytes.len() < 16384);
        String::from_utf8(bytes).unwrap()
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if matches!(self.process.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = self.process.kill();
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            if matches!(self.process.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // The enclosing Windows Job remains the primary descendant containment.
        assert!(
            std::thread::panicking(),
            "owned Chat child cleanup insufficient"
        );
    }
}
fn env_path(name: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os(name).expect("owned process input"));
    assert!(path.is_absolute());
    path
}
pub fn run_worker() {
    let root = env_path("NF_CHAT_LOCAL_PROCESS_ROOT");
    let mode = std::env::var("NF_CHAT_LOCAL_PROCESS_MODE").unwrap();
    let who = std::env::var("NF_CHAT_LOCAL_PROCESS_WHO").unwrap();
    assert!(matches!(who.as_str(), "alice" | "bob"));
    let address = env_path("NF_CHAT_LOCAL_PROCESS_ADDRESS");
    let stop = env_path("NF_CHAT_LOCAL_PROCESS_STOP");
    assert!(address.parent() == Some(root.as_path()) && stop.parent() == Some(root.as_path()));
    let vault =
        PrivateVault::open(&root.join(format!("{who}-private")), &root.join("saves")).unwrap();
    let peer = TransportIdentity::load(&vault).unwrap().peer_id();
    let local = vault.load_identity(peer.to_bytes()).unwrap();
    let state =
        nf_identity::codec::decode_state(&fs::read(root.join("membership.bin")).unwrap()).unwrap();
    let actor = Author {
        account: local.public.account,
        device: local.public.device,
    };
    let policy = ChatPolicy { scope: state.scope };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if mode == "receiver" {
        let store = ChatStore::open_existing(
            root.join(format!("receiver-{who}.sqlite")),
            &policy,
            KnownChatFrontiers {
                scope: state.scope,
                revision: 0,
                membership_revision: 1,
            },
        )
        .unwrap();
        let mut server = ChatPeerServer::new(store, &vault, policy).unwrap();
        runtime.block_on(async {
            let until=tokio::time::Instant::now()+Duration::from_secs(30);
            loop {
                tokio::select! {
                    _=tokio::time::sleep_until(until)=>panic!("owned sidecar whole lifetime deadline"),
                    _=tokio::time::sleep(Duration::from_millis(5))=>{if stop.is_file(){break;}},
                    event=server.next()=>{
                        if let ChatPeerServerEvent::Ready{address:bound,..}=event.unwrap() {
                            let bytes=bound.to_string();assert!(bytes.len()<256);
                            let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&address).unwrap();
                            use std::io::Write;file.write_all(bytes.as_bytes()).unwrap();file.sync_all().unwrap();
                        }
                    }
                }
            }
        });
        let store = server.into_store();
        assert_eq!(store.known_frontiers().unwrap().revision, 1);
        drop(store);
        println!("NF_CHAT_SIDECAR_STOPPED");
    } else {
        assert_eq!(mode, "client");
        let remote = state
            .devices
            .iter()
            .find(|(_, device)| device.account != actor.account)
            .unwrap();
        let remote = Author {
            account: remote.1.account,
            device: *remote.0,
        };
        let profile = OutboxProfile::from_current(&policy, &state, actor, remote).unwrap();
        let mut outbox =
            ClientOutbox::open_existing(root.join(format!("sender-{who}.sqlite")), &profile, 1)
                .unwrap();
        let id = if who == "alice" { 81 } else { 82 };
        let original = outbox.entry([id; 16]).unwrap().unwrap();
        let pin = ChatPeerPin::from_current(&state, remote).unwrap();
        let dial = fs::read_to_string(&address).unwrap();
        assert!(dial.len() < 256);
        runtime.block_on(async {
            let delivered = deliver_pending(
                &mut outbox,
                &vault,
                &state,
                &pin,
                dial.parse().unwrap(),
                [id; 16],
            )
            .await
            .unwrap();
            assert_eq!(delivered.signed, original.signed);
            assert_eq!(
                delivered.original_request,
                RequestId::from_bytes([id + 20; 16])
            );
            assert!(matches!(delivered.state, OutgoingState::Delivered(_)));
            assert_eq!(outbox.known_revision(), Ok(2));
            let query = HistoryQuery {
                request: RequestId::from_bytes([id + 30; 16]),
                reader: actor,
                channel: Channel::General,
                after_cursor: 0,
                limit: 1,
            };
            let page = fetch_history(&vault, &state, &pin, dial.parse().unwrap(), query)
                .await
                .unwrap();
            assert_eq!(page.entries.len(), 1);
            assert_eq!(page.next_cursor, 1);
            assert_eq!(page.entries[0].signed, original.signed);
        });
        println!("NF_CHAT_CLIENT_DELIVERED_HISTORY_ONE");
    }
}
