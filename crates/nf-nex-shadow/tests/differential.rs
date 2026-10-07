mod support;
use nf_nex_shadow::*;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Owned {
    child: Option<Child>,
    root: PathBuf,
    base: PathBuf,
}
impl Drop for Owned {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.root.starts_with(&self.base) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
fn oracle(mode: &str, corpus: &str) -> String {
    let java = std::env::var_os("NF_SHADOW_JAVA").expect("explicit Java executable required");
    let classpath_file =
        std::env::var_os("NF_SHADOW_CLASSPATH_FILE").expect("explicit oracle classpath required");
    let classpath = fs::read_to_string(classpath_file).unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65536);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    let root = base.join(format!(
        "oracle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    assert!(root.starts_with(&base));
    fs::create_dir(&root).unwrap();
    let mut owned = Owned {
        child: None,
        root,
        base,
    };
    let output = owned.root.join("out");
    let errors = owned.root.join("err");
    let out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    let err = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&errors)
        .unwrap();
    let mut command = Command::new(java);
    command
        .args([
            "-cp",
            classpath.trim(),
            "nf.nex.reference.DifferentialOracleMain",
            mode,
        ])
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../java/nex-reference/src/test/resources")
                .join(corpus),
        )
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    owned.child = Some(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            fs::metadata(&output).unwrap().len() <= 65536
                && fs::metadata(&errors).unwrap().len() <= 4096,
            "bounded oracle output"
        );
        if let Some(status) = owned.child.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success(), "oracle failed");
            break;
        }
        assert!(Instant::now() < deadline, "bounded oracle deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(fs::metadata(&output).unwrap().len() <= 65536);
    assert_eq!(fs::metadata(&errors).unwrap().len(), 0);
    let result = fs::read_to_string(output).unwrap();
    assert!(!result.is_empty());
    result
}
/// Explicit real JVM gate; run through package-native verification ownership on Windows.
#[test]
#[ignore = "requires NF_SHADOW_JAVA and NF_SHADOW_CLASSPATH_FILE; exportDifferentialClasspath first"]
fn real_java_oracle_matches_rust_from_same_raw_inputs_without_expected_column_oracle() {
    let mut expected = String::new();
    for row in include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .filter(|r| !r.starts_with('#') && !r.is_empty())
    {
        let f: Vec<_> = row.split('|').collect();
        let input = support::war(&f);
        let result = evaluate_war(&input, WarOperation::Generate).unwrap();
        expected.push_str(&format!(
            "war|{}|{}|{}|{}|{}|{}|{}\n",
            f[0],
            result.generated,
            result.ended,
            result.abort_current_action,
            result.valid,
            support::modifiers(&result.existing_priority),
            support::modifiers(&result.writes)
        ));
    }
    assert_eq!(oracle("war", "war-v1.tsv"), expected);
    let mut expected = String::new();
    for row in include_str!("../../../java/nex-reference/src/test/resources/peace-v1.tsv")
        .lines()
        .filter(|r| !r.starts_with('#') && !r.is_empty())
    {
        let f: Vec<_> = row.split('|').collect();
        let result = evaluate_selected_peace(&support::peace(&f)).unwrap();
        expected.push_str(&format!(
            "peace|{}|{}|{}|{}\n",
            f[0],
            match result.decision {
                PeaceDecision::None => "NONE",
                PeaceDecision::Ceasefire => "CEASEFIRE",
                PeaceDecision::Treaty => "TREATY",
            },
            result.consumed_draws,
            support::effects(&result.effects)
        ));
    }
    assert_eq!(oracle("peace", "peace-v1.tsv"), expected);
}
