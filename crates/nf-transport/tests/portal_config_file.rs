use libp2p::identity;
use nf_transport::{
    PeerError,
    portal_config::{PortalMode, read_config},
};
use std::{fs, path::PathBuf};
struct Scratch {
    path: PathBuf,
    parent: PathBuf,
}
impl Scratch {
    fn new() -> Self {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).unwrap();
        let tag: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let path = parent.join(format!("nf-portal-config-{tag}"));
        fs::create_dir(&path).unwrap();
        Self {
            path: path.canonicalize().unwrap(),
            parent,
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.path.parent() == Some(self.parent.as_path())
            && self
                .path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("nf-portal-config-"))
            && self.path.canonicalize().ok().as_ref() == Some(&self.path)
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn serve() -> String {
    let peer = identity::Keypair::ed25519_from_bytes([0x31; 32])
        .unwrap()
        .public()
        .to_peer_id();
    format!(
        "NF-PORTAL-CONFIG-1\nmode serve\nuniverse {}\nhistory {}\nruleset {}\ncontent {}\nlocal_account {}\nlocal_device {}\nlocal_event_min 7\nlocal_store_min 9\nlocal_membership_min 3\nserver_peer {peer}\nserver_account {}\nserver_device {}\nserver_membership_min 5\nreceipt_listen /ip4/127.0.0.1/tcp/0\nbulk_listen /ip4/127.0.0.1/tcp/0\nnotification_listen /ip4/127.0.0.1/tcp/0\nanchor_count 0\nEND\n",
        "01".repeat(16),
        "02".repeat(16),
        "03".repeat(32),
        "04".repeat(32),
        "05".repeat(16),
        "06".repeat(16),
        "07".repeat(16),
        "08".repeat(16)
    )
}
#[test]
fn opened_regular_config_preserves_input_and_adjacent_state() {
    let scratch = Scratch::new();
    let config = scratch.path.join("operator.cfg");
    let database = scratch.path.join("untouched.db");
    let original = serve();
    fs::write(&config, original.as_bytes()).unwrap();
    fs::write(
        &database,
        b"not a SQLite database; parsing grants no Store access",
    )
    .unwrap();
    let state = fs::read(&database).unwrap();
    let parsed = read_config(&config, PortalMode::Serve).unwrap();
    assert_eq!(parsed.local.store_revision, 9);
    assert_eq!(parsed.local.membership_revision, Some(3));
    assert_eq!(fs::read(&config).unwrap(), original.as_bytes());
    assert_eq!(fs::read(&database).unwrap(), state);
}
#[test]
fn missing_directory_invalid_and_oversized_inputs_are_preserved_and_refused() {
    let scratch = Scratch::new();
    let missing = scratch.path.join("missing.cfg");
    assert_eq!(
        read_config(&missing, PortalMode::Serve).unwrap_err(),
        PeerError::Storage
    );
    assert!(!missing.exists());
    assert_eq!(
        read_config(&scratch.path, PortalMode::Serve).unwrap_err(),
        PeerError::Storage
    );
    let config = scratch.path.join("operator.cfg");
    for bytes in [
        vec![b'x'; 16_384],
        vec![b'x'; 16_385],
        vec![0xff, b'\n'],
        serve().replace('\n', "\r\n").into_bytes(),
    ] {
        fs::write(&config, &bytes).unwrap();
        let result = read_config(&config, PortalMode::Serve);
        assert!(result.is_err());
        if bytes.len() == 16_385 {
            assert_eq!(result.unwrap_err(), PeerError::Limit);
        }
        assert_eq!(fs::read(&config).unwrap(), bytes);
    }
}
#[cfg(unix)]
#[test]
fn symlink_and_nonregular_config_refuse_without_following_or_waiting() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new();
    let target = scratch.path.join("real.cfg");
    let link = scratch.path.join("linked.cfg");
    let body = serve();
    fs::write(&target, body.as_bytes()).unwrap();
    symlink(&target, &link).unwrap();
    assert_eq!(
        read_config(&link, PortalMode::Serve).unwrap_err(),
        PeerError::Storage
    );
    assert_eq!(fs::read(&target).unwrap(), body.as_bytes());
    assert!(read_config(std::path::Path::new("/dev/null"), PortalMode::Serve).is_err());
}
