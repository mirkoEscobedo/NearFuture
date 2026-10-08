use super::protocol::{self, Cut, Descriptor};
use nf_store::supplies::{ChallengeRequest, ProofAttempt, SuppliesStore};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

pub struct OwnedWorker {
    child: Child,
    root: PathBuf,
    reaped: bool,
}
impl OwnedWorker {
    pub fn spawn(root: &Path) -> Self {
        protocol::directory(root);
        let mut command =
            Command::new(std::env::current_exe().expect("current worker test binary"));
        command
            .args(["--ignored", "--exact", "supplies_crash_worker_dispatcher"])
            .env("NF_SUPPLIES_WORKER_ROOT", root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // Hidden child, no breakaway flag; qualified outer Job retains the tree.
        }
        let child = command
            .spawn()
            .unwrap_or_else(|_| panic!("exact owned worker spawn"));
        Self {
            child,
            root: root.to_owned(),
            reaped: false,
        }
    }
    fn active(&mut self) {
        let status = self
            .child
            .try_wait()
            .unwrap_or_else(|_| panic!("owned worker status"));
        if status.is_some() {
            self.reaped = true;
        }
        if status.is_some() {
            panic!(
                "owned worker exited before prerequisite; closed stage/family/detail {:?}",
                protocol::failure(&self.root)
            );
        }
    }
    pub fn frame<const N: usize>(&mut self, name: &str, until: Instant) -> [u8; N] {
        loop {
            self.active();
            assert!(Instant::now() < until, "owned worker prerequisite deadline");
            if let Some(bytes) = protocol::read_frame(&self.root.join(name)) {
                self.active();
                return bytes;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn reap_until(&mut self, until: Instant) -> Option<ExitStatus> {
        while Instant::now() < until {
            if let Some(status) = self
                .child
                .try_wait()
                .unwrap_or_else(|_| panic!("owned worker reap status"))
            {
                self.reaped = true;
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }
    pub fn kill_and_reap(&mut self) {
        self.active();
        self.child
            .kill()
            .unwrap_or_else(|_| panic!("exact owned worker kill"));
        let status = self
            .reap_until(Instant::now() + Duration::from_secs(3))
            .expect("owned worker must be reaped within cleanup bound");
        assert!(!status.success(), "actual forced child termination");
        assert!(self.reaped);
    }
}
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.reap_until(Instant::now() + Duration::from_secs(3));
            assert!(
                self.reaped,
                "owned worker cleanup incomplete; outer qualified Job must terminate the tree"
            );
        }
    }
}
// Management-only dispatcher; no parent key or private AuthTicket is serialized.
pub fn run_worker(root: &Path) {
    protocol::directory(root);
    assert!(
        root.is_absolute(),
        "explicit owned absolute worker directory"
    );
    let descriptor = Descriptor::read(root);
    let mut store = protocol::actual(
        root,
        1,
        SuppliesStore::open_existing(
            root.join("supplies.sqlite"),
            &protocol::policy(&descriptor.issue),
            descriptor.known,
        ),
    );
    assert_eq!(
        protocol::actual(root, 2, store.known_frontiers()),
        descriptor.known
    );
    protocol::write_new(&root.join("prepared"), protocol::PREPARED);
    protocol::write_new(&root.join("before"), protocol::BEFORE);
    assert_eq!(
        &protocol::await_frame::<8>(&root.join("go"), Instant::now() + protocol::MANAGEMENT),
        protocol::GO
    );
    assert!(
        descriptor.cut == Cut::AfterReturn,
        "before-call child must never receive GO"
    );
    let before_challenge = Instant::now();
    let mut issued = protocol::actual(
        root,
        3,
        store.issue_challenge(ChallengeRequest::Issue(&descriptor.issue)),
    );
    assert_eq!(issued.template.peer, descriptor.peer);
    protocol::write_new(
        &root.join("template"),
        &protocol::encode_template(&issued.template),
    );
    issued.template.signature = protocol::await_frame::<64>(
        &root.join("signature"),
        before_challenge + Duration::from_secs(5),
    );
    let outcome = protocol::actual(
        root,
        4,
        store.issue(
            &descriptor.issue,
            ProofAttempt {
                ticket: issued.ticket,
                proof: issued.template,
            },
        ),
    )
    .expect("actual successful worker outcome");
    protocol::write_new(&root.join("returned"), &protocol::returned(outcome));
    // Some already returned. Parent withholds its separate wrapper ACK and kills this exact child.
    let _ = protocol::await_frame::<8>(&root.join("ack"), Instant::now() + protocol::MANAGEMENT);
    panic!("management worker must be killed and reaped before wrapper acknowledgment");
}
