use nf_identity::private_storage::PrivateVault;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Owned {
    root: PathBuf,
    child: Option<Child>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_client(mode: &str) {
    let java = std::env::var_os("NF_IPC_JAVA").expect("NF_IPC_JAVA required");
    let classpath = fs::read_to_string(
        std::env::var_os("NF_IPC_JAVA_CLASSPATH_FILE").expect("classpath file required"),
    )
    .unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65536);
    let root = std::env::temp_dir().join(format!("nf-parent-java-{}-{mode}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut owned = Owned {
        root: root.clone(),
        child: None,
    };
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault_path = root.join("vault");
    let vault = PrivateVault::create_diagnosed(&vault_path, &saves)
        .unwrap_or_else(|failure| panic!("private vault create failed: {failure:?}"));
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_nf-ipc-node"));
    let executable = if mode == "stalled-child" {
        let target = root.join(format!("stalled-parent{}", std::env::consts::EXE_SUFFIX));
        let mut compile = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            compile.creation_flags(0x0800_0000);
        }
        compile
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/stalled_parent.rs"))
            .arg("--edition=2024")
            .arg("-o")
            .arg(&target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        owned.child = Some(compile.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(15);
        while owned.child.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "benign fixture compilation deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = owned.child.take().unwrap().wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "benign fixture compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        target
    } else {
        executable
    };
    assert!(executable.is_absolute());
    let digest = Sha256::digest(fs::read(&executable).unwrap());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut command = Command::new(java);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .args([
            "-cp",
            classpath.trim(),
            "nf.adapter.ipc.ParentNodeProcessClient",
        ])
        .arg(&executable)
        .arg(&vault_path)
        .arg(&saves)
        .arg(hex)
        .arg(mode)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    owned.child = Some(command.spawn().unwrap());
    // Startup15s plus cleanup10s are independently bounded by the Java owner.
    let deadline = Instant::now()
        + Duration::from_secs(if mode.starts_with("lifecycle") {
            50
        } else {
            30
        });
    while owned.child.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "owned Java parent exceeded its outer deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = owned.child.take().unwrap().wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "parent owner missing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "PARENT_NODE_AUTH_SHUTDOWN_OK"
    );
    assert!(output.stderr.is_empty());
    assert!(
        vault
            .read_optional_private_blob("java-parent-owner")
            .unwrap()
            .is_none()
    );
}

/// Required explicit process lane; never credit ordinary ignored execution as coverage.
#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_launches_authenticates_both_rust_roles_and_reaps_on_close() {
    run_client("normal");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_refuses_wrong_executable_pin_without_publication() {
    run_client("wrong-pin");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_refuses_cancelled_launch_without_publication() {
    run_client("cancelled-before");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_revokes_active_generation_then_reaps_only_owned_child() {
    run_client("cancelled-active");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_cleans_up_when_owner_fails_after_publication() {
    run_client("owner-error");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_lifecycle_renews_and_fences_campaign_changes_without_callback_waits() {
    run_client("lifecycle");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_lifecycle_revokes_authentication_when_each_owned_child_dies() {
    run_client("lifecycle-failure");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_background_owner_times_out_a_stalled_child_and_forcibly_reaps_it() {
    run_client("stalled-child");
}

#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn abrupt_java_parent_death_closes_private_pipe_and_reaps_authenticated_rust_child() {
    run_client("abrupt-jvm");
}
