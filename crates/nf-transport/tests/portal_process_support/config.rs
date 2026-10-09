use super::ProcessFixture;
use libp2p::Multiaddr;
use nf_transport::portal_config::{PortalConfig, PortalMode};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
impl ProcessFixture {
    pub fn write_configs(
        &self,
        listeners: &[Multiaddr; 3],
        pinned: &[Multiaddr; 3],
    ) -> (PathBuf, PathBuf) {
        let serve = self.config_text(false, listeners);
        let watch = self.config_text(true, pinned);
        PortalConfig::parse(serve.as_bytes(), PortalMode::Serve).unwrap();
        PortalConfig::parse(watch.as_bytes(), PortalMode::Watch).unwrap();
        let server_path = self.root.join("server.conf");
        let watch_path = self.root.join("watch.conf");
        std::fs::write(&server_path, serve).unwrap();
        std::fs::write(&watch_path, watch).unwrap();
        (server_path, watch_path)
    }
    fn config_text(&self, watch: bool, addresses: &[Multiaddr; 3]) -> String {
        let (mode, local, account, device, keys, rows) = if watch {
            let original = self.original.original();
            let row = format!(
                "anchor_count 1\nanchor 0 {} {} {} {} {} {} {} {} {}\n",
                self.anchor.generation,
                hex(&self.anchor.digest),
                hex(self.original.request().as_bytes()),
                hex(self.original.operation().as_bytes()),
                original.operation_kind,
                hex(&original.payload_digest),
                self.anchor.minimum.event.0,
                self.anchor.minimum.store_revision,
                self.anchor.minimum.membership_revision
            );
            (
                "watch",
                self.client_known,
                self.client_account,
                self.client_device,
                ["receipt_address", "bulk_address", "notification_address"],
                row,
            )
        } else {
            (
                "serve",
                self.server_known,
                self.server_account,
                self.server_device,
                ["receipt_listen", "bulk_listen", "notification_listen"],
                "anchor_count 0\n".to_owned(),
            )
        };
        format!(
            "NF-PORTAL-CONFIG-1\nmode {mode}\nuniverse {}\nhistory {}\nruleset {}\ncontent {}\nlocal_account {}\nlocal_device {}\nlocal_event_min {}\nlocal_store_min {}\nlocal_membership_min {}\nserver_peer {}\nserver_account {}\nserver_device {}\nserver_membership_min 1\n{} {}\n{} {}\n{} {}\n{rows}END\n",
            hex(local.scope.universe.as_bytes()),
            hex(local.scope.history.as_bytes()),
            hex(&[4; 32]),
            hex(&[5; 32]),
            hex(account.as_bytes()),
            hex(device.as_bytes()),
            local.event_sequence.0,
            local.store_revision,
            local.membership_revision.unwrap(),
            self.server_peer,
            hex(self.server_account.as_bytes()),
            hex(self.server_device.as_bytes()),
            keys[0],
            addresses[0],
            keys[1],
            addresses[1],
            keys[2],
            addresses[2]
        )
    }
}
/// The returned timestamp is before file publication, hence before the child's first owner deadline.
pub fn publish_start(path: &Path, addresses: &[Multiaddr; 3]) -> Instant {
    let frame = format!(
        "NF-PORTAL-START-1\n{}\n{}\n{}\nEND\n",
        addresses[0], addresses[1], addresses[2]
    );
    let temporary = path.with_extension("staging");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .unwrap();
    file.write_all(frame.as_bytes()).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(!path.exists());
    let before_publication = Instant::now();
    std::fs::rename(temporary, path).unwrap();
    before_publication
}
