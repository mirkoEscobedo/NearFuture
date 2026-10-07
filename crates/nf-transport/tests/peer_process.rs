#[path = "support/child.rs"]
mod child;
#[path = "support/community.rs"]
mod community;
#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/retained_store.rs"]
mod retained_store;
#[path = "support/scratch.rs"]
mod scratch;
use nf_store::KnownFrontiers;
use std::{path::Path, time::Duration};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn config(
    path: &Path,
    known: KnownFrontiers,
    remote: Option<(&owned_community::OwnedCommunity, &str)>,
) {
    let mut s = format!(
        "NF-PEER-CONFIG-1\nuniverse={}\nhistory={}\nruleset={}\ncontent={}\nevent_sequence={}\nstore_revision={}\nmembership_revision={}\n",
        hex(known.scope.universe.as_bytes()),
        hex(known.scope.history.as_bytes()),
        hex(&[4; 32]),
        hex(&[5; 32]),
        known.event_sequence.0,
        known.store_revision,
        known.membership_revision.unwrap()
    );
    if let Some((c, address)) = remote {
        s += &format!(
            "server_peer={}\nserver_account={}\nserver_device={}\ncontrol_address={}\n",
            libp2p::PeerId::from_bytes(&c.server.public.peer).unwrap(),
            hex(c.server.public.account.as_bytes()),
            hex(c.server.public.device.as_bytes()),
            address
        );
    }
    std::fs::write(path, s).unwrap();
}
fn args(root: &Path, mode: &str, config: &Path, last: &str) -> Vec<String> {
    let who = if mode == "serve" { "server" } else { "client" };
    vec![
        mode.into(),
        root.join(format!("{who}-private")).to_str().unwrap().into(),
        root.join("saves").to_str().unwrap().into(),
        root.join(if who == "server" {
            "server-pending.sqlite"
        } else {
            "client.sqlite"
        })
        .to_str()
        .unwrap()
        .into(),
        config.to_str().unwrap().into(),
        last.into(),
    ]
}
#[test]
fn actual_foreground_peers_reconnect_with_stable_private_identity_and_retained_request() {
    let tmp = scratch::Scratch::new();
    let c = owned_community::OwnedCommunity::new(&tmp.0);
    let pending = retained_store::create_from(
        &c.state,
        &c.server.public,
        &c.client.public,
        &tmp.0.join("server-pending.sqlite"),
        1,
        false,
    );
    let sk = pending.known_frontiers().unwrap();
    drop(pending);
    let ck = c.client_store.known_frontiers().unwrap();
    let sc = tmp.0.join("server.cfg");
    let cc = tmp.0.join("client.cfg");
    config(&sc, sk, None);
    drop(c.server_store);
    drop(c.client_store);
    // Both durable owners are closed before subprocess ownership starts. Public identity is kept separately.
    let expected = libp2p::PeerId::from_bytes(&c.server.public.peer)
        .unwrap()
        .to_string();
    let mut first = child::OwnedChild::spawn(&args(&tmp.0, "serve", &sc, "12000"), &tmp.0, "first");
    let (address, _) = first.ready(Duration::from_secs(8));
    assert!(address.ends_with(&expected));
    // Config helper requires only public fields, not the moved stores.
    let extra = format!(
        "server_peer={}\nserver_account={}\nserver_device={}\ncontrol_address={}\n",
        expected,
        hex(c.server.public.account.as_bytes()),
        hex(c.server.public.device.as_bytes()),
        address
    );
    config(&cc, ck, None);
    let mut s = std::fs::read_to_string(&cc).unwrap();
    s += &extra;
    std::fs::write(&cc, s).unwrap();
    let mut query = child::OwnedChild::spawn(
        &args(&tmp.0, "query", &cc, &hex(&[9; 16])),
        &tmp.0,
        "query1",
    );
    assert!(
        query.wait(Duration::from_secs(8)).success(),
        "{}",
        query.error()
    );
    assert_eq!(query.output().trim(), "NF_PEER_STATUS PENDING");
    first.terminate();
    let mut second =
        child::OwnedChild::spawn(&args(&tmp.0, "serve", &sc, "12000"), &tmp.0, "second");
    let (address, _) = second.ready(Duration::from_secs(8));
    assert!(address.ends_with(&expected));
    config(&cc, ck, None);
    let mut s = std::fs::read_to_string(&cc).unwrap();
    s += &format!(
        "server_peer={}\nserver_account={}\nserver_device={}\ncontrol_address={}\n",
        expected,
        hex(c.server.public.account.as_bytes()),
        hex(c.server.public.device.as_bytes()),
        address
    );
    std::fs::write(&cc, s).unwrap();
    let mut query = child::OwnedChild::spawn(
        &args(&tmp.0, "query", &cc, &hex(&[9; 16])),
        &tmp.0,
        "query2",
    );
    assert!(
        query.wait(Duration::from_secs(8)).success(),
        "{}",
        query.error()
    );
    assert_eq!(query.output().trim(), "NF_PEER_STATUS PENDING");
    assert!(
        second.wait(Duration::from_secs(14)).success(),
        "{}",
        second.error()
    );
    assert!(second.output().contains("NF_PEER_STOPPED"));
}
