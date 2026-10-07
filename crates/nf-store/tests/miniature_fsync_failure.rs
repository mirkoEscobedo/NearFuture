#[path = "miniature_support/crash_data.rs"]
mod crash_data;
mod miniature_support;
#[path = "miniature_support/process.rs"]
mod process_support;
#[path = "miniature_support/sync_trace.rs"]
mod sync_trace;
#[path = "miniature_support/vault.rs"]
mod vault_support;
use nf_contract::identity::*;
use nf_kernel::miniature::*;
use nf_store::{KnownFrontiers, StoreError, miniature::*};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
#[test]
fn miniature_sync_fault_worker() {
    let Some(path) = std::env::var_os("NF_MINIATURE_SYNC_DB") else {
        return;
    };
    let mode = std::env::var("NF_MINIATURE_SYNC_MODE").unwrap();
    let result = PathBuf::from(std::env::var_os("NF_MINIATURE_SYNC_RESULT").unwrap());
    let known = MiniatureKnownFrontiers {
        storage: KnownFrontiers::genesis(
            UniverseId::from_bytes([1; 16]),
            HistoryId::from_bytes([2; 16]),
        ),
        minimum_authority_term: AuthorityTerm(0),
    };
    let mut store = MiniatureStore::open_existing(path, known, AuthConfig::default()).unwrap();
    let vault = PathBuf::from(std::env::var_os("NF_MINIATURE_SYNC_VAULT").unwrap());
    let saves = PathBuf::from(std::env::var_os("NF_MINIATURE_SYNC_SAVES").unwrap());
    let mut signer = vault_support::load_signer(&mut store, &vault, &saves);
    vault_support::fresh_claim(&mut store, &mut signer);
    vault_support::fresh_resume(&mut store, &mut signer);
    vault_support::activity(&mut store, &mut signer);
    let batch = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let proof = vault_support::owner_attempt(&mut store, &mut signer, ChallengeRequest::Commit);
    crash_data::write_before(&store, &result);
    std::io::stdout()
        .write_all(b"MINIATURE_ADVANCE_COMMIT_READY\n")
        .unwrap();
    std::io::stdout().flush().unwrap();
    match mode.as_str() {
        "calibration" => {
            store.commit(&batch, proof).unwrap();
            std::fs::write(result, b"calibration-ack").unwrap();
        }
        "fault" => {
            assert_eq!(
                store.commit(&batch, proof),
                Err(MiniatureStoreError::Storage(StoreError::Io))
            );
            assert!(matches!(
                store.snapshot_bytes(),
                Err(MiniatureStoreError::Storage(StoreError::Quarantined))
            ));
            std::fs::write(result, b"commit-refused-no-ack").unwrap();
        }
        _ => panic!("closed actual sync test mode"),
    }
}
fn trace_worker(
    f: &vault_support::VaultFixture,
    db: &Path,
    trace: &Path,
    result: &Path,
    mode: &str,
    cut: Option<&sync_trace::SyncCut>,
) {
    let mut command = Command::new("strace");
    command
        .args([
            "--kill-on-exit",
            "-f",
            "-yy",
            "-s",
            "128",
            "-e",
            "trace=fsync,fdatasync,write",
            "-o",
        ])
        .arg(trace);
    if let Some(cut) = cut {
        command.args([
            "-e",
            &format!("inject={}:error=EIO:when={}", cut.syscall, cut.occurrence),
        ]);
    }
    command
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "miniature_sync_fault_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NF_MINIATURE_SYNC_DB", db)
        .env("NF_MINIATURE_SYNC_RESULT", result)
        .env("NF_MINIATURE_SYNC_MODE", mode)
        .env("NF_MINIATURE_SYNC_VAULT", &f.vault)
        .env("NF_MINIATURE_SYNC_SAVES", &f.saves);
    let mut child = process_support::OwnedChild::spawn(&mut command);
    assert!(
        child.wait(Duration::from_secs(30)).success(),
        "required calibrated actual advancing sync worker failed"
    );
}
#[test]
fn real_linux_sync_failure_refuses_miniature_commit_and_recovers_pending() {
    if !cfg!(target_os = "linux")
        || std::env::var_os("NF_MINIATURE_REQUIRE_SYNC_FAULT").as_deref()
            != Some(std::ffi::OsStr::new("1"))
    {
        eprintln!(
            "UNOBSERVED: Linux NF_MINIATURE_REQUIRE_SYNC_FAULT=1 and maintained strace are required"
        );
        return;
    }
    let mut f = vault_support::VaultFixture::new();
    let original = f.scratch.0.join("pending.sqlite");
    let mut store = f.create(&original);
    vault_support::fresh_claim(&mut store, &mut f.signer);
    let intent = miniature_support::intent(store.world(), &f.policy.owner, 7);
    let proof = vault_support::owner_attempt(
        &mut store,
        &mut f.signer,
        ChallengeRequest::Prepare(&intent),
    );
    store.prepare(vec![intent.clone()], vec![proof]).unwrap();
    drop(store);
    let calibration = f.scratch.0.join("calibration.sqlite");
    let fault = f.scratch.0.join("fault.sqlite");
    std::fs::copy(&original, &calibration).unwrap();
    std::fs::copy(&original, &fault).unwrap();
    let ct = f.scratch.0.join("calibration.trace");
    let cr = f.scratch.0.join("calibration.result");
    trace_worker(&f, &calibration, &ct, &cr, "calibration", None);
    assert_eq!(
        process_support::read_bounded(&cr, 32).unwrap(),
        b"calibration-ack"
    );
    let trace = String::from_utf8(process_support::read_bounded(&ct, 1_048_576).unwrap()).unwrap();
    let cut = sync_trace::target(&trace, "calibration.sqlite");
    let ft = f.scratch.0.join("fault.trace");
    let fr = f.scratch.0.join("fault.result");
    trace_worker(&f, &fault, &ft, &fr, "fault", Some(&cut));
    let trace = String::from_utf8(process_support::read_bounded(&ft, 1_048_576).unwrap()).unwrap();
    sync_trace::assert_injected(&trace, "fault.sqlite", &cut);
    assert_eq!(
        process_support::read_bounded(&fr, 32).unwrap(),
        b"commit-refused-no-ack"
    );
    let known = crash_data::known(&fr);
    let restored = MiniatureStore::open_existing(&fault, known, f.policy.auth).unwrap();
    assert!(restored.pending().is_some());
    assert_eq!(restored.outbox().unwrap().len(), 0);
    assert_eq!(
        restored.snapshot_bytes().unwrap(),
        process_support::read_bounded(&fr.with_extension("before.envelope"), 1_048_576).unwrap()
    );
    assert_eq!(
        encode_miniature_snapshot(restored.world()).unwrap(),
        process_support::read_bounded(&fr.with_extension("before.world"), 1_048_576).unwrap()
    );
    assert!(matches!(
        restored
            .query_bound(intent.request, intent.actor, intent.device, f.policy.scope)
            .unwrap()
            .unwrap()
            .status,
        MiniatureRequestStatus::Pending { .. }
    ));
}
#[test]
fn calibration_counts_only_exact_worker_and_syscall_after_successful_setup() {
    let text = "11 fsync(3<calibration.sqlite>) = 0\n22 fsync(3<other>) = 0\n11 fdatasync(3<calibration.sqlite>) = 0\n11 write(1, \"MINIATURE_ADVANCE_COMMIT_READY\\n\", 31) = 31\n22 fsync(3<calibration.sqlite>) = 0\n11 fsync(3<calibration.sqlite-journal>) = 0\n";
    let cut = sync_trace::target(text, "calibration.sqlite");
    assert_eq!(cut.occurrence, 2);
    assert_eq!(cut.tid, 11);
    assert!(cut.journal);
    sync_trace::assert_injected(
        &text.replace("calibration.sqlite", "fault.sqlite").replace(
            "11 fsync(3<fault.sqlite-journal>) = 0",
            "11 fsync(3<fault.sqlite-journal>) = -1 EIO (INJECTED)",
        ),
        "fault.sqlite",
        &cut,
    );
}
