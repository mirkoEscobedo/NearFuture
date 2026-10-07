use super::*;
use std::{fs, path::PathBuf};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&base).unwrap();
        let name: String = crate::keys::random_id()
            .unwrap()
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect();
        let root = base.join(format!("batch-io-{name}"));
        assert!(root.starts_with(&base));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn script(&self, text: &str) -> PathBuf {
        let path = self.0.join("fixture.ps1");
        fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn early_helper_exit_unblocks_owned_full_pipe_writer() {
    let root = Scratch::new();
    assert_eq!(run_with_reason(&root.script("exit 0"), vec![]), Ok(()));
    let start = Instant::now();
    assert_eq!(
        run(&root.script("exit 2"), vec![b'x'; INPUT_LIMIT]),
        Err(IdentityError::PrivateStorage)
    );
    assert!(start.elapsed() < Duration::from_secs(10));
}
#[test]
fn unexpected_helper_output_is_a_refusal() {
    let root = Scratch::new();
    assert_eq!(
        run(
            &root.script("Write-Output 'public fixture noise'; exit 0"),
            vec![]
        ),
        Err(IdentityError::PrivateStorage)
    );
}
#[test]
fn actual_five_second_timeout_kills_helper_before_scoped_pipe_join() {
    let root = Scratch::new();
    let start = Instant::now();
    assert_eq!(
        run(
            &root.script("Start-Sleep -Seconds 30"),
            vec![b'x'; INPUT_LIMIT]
        ),
        Err(IdentityError::PrivateStorage)
    );
    assert!(start.elapsed() >= Duration::from_secs(5));
    assert!(start.elapsed() < Duration::from_secs(10));
}
#[test]
fn actual_sidecar_rejects_malformed_count_trailing_and_oversized_lines() {
    let script = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/private-acl-batch.ps1"
    ));
    for input in [
        b"0\n".to_vec(),
        b"66\n".to_vec(),
        b"1\n\n".to_vec(),
        b"1\npath\ntrailing\n".to_vec(),
        [
            b"1\n".as_slice(),
            vec![b'x'; 4097].as_slice(),
            b"\n".as_slice(),
        ]
        .concat(),
    ] {
        assert_eq!(run(script, input), Err(IdentityError::PrivateStorage));
    }
}
