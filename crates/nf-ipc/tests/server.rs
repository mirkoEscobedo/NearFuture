use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::private_storage::PrivateVault;
use nf_ipc::{
    FramePump, LocalPrincipal, NoOperations, NodeServer, PublishedRendezvous, SessionConfig,
};
use nf_wire::generated as g;
use prost::Message;
use std::{
    fs,
    net::TcpStream,
    time::{Duration, Instant},
};
#[test]
fn real_server_authenticates_then_answers_query_without_claiming_success() {
    let root = std::env::temp_dir().join(format!("nf-ipc-node-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("vault"), &saves).unwrap();
    let config = SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits: nf_ipc::default_limits(),
    };
    let principal = LocalPrincipal {
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
    };
    let mut node = NodeServer::bind(config.clone(), principal, NoOperations).unwrap();
    let publication =
        PublishedRendezvous::publish(&vault, "ipc-node", config, principal, node.address())
            .unwrap();
    node.install_authenticator(
        publication
            .record()
            .authenticator(nf_ipc::EndpointRole::Control)
            .unwrap(),
    )
    .unwrap();
    let mut peer = TcpStream::connect(node.address()).unwrap();
    let mut client = FramePump::new(peer.try_clone().unwrap(), 65_536).unwrap();
    let hello = publication
        .record()
        .client(nf_ipc::EndpointRole::Control)
        .unwrap();
    client.send(hello.hello()).unwrap();
    let start = Instant::now();
    let challenge = loop {
        node.poll(Instant::now()).unwrap();
        if let Some(bytes) = client.poll(4096, 4096).unwrap() {
            break bytes;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
    };
    let proof = hello.respond(&challenge).unwrap();
    client.send(proof.proof()).unwrap();
    let start = Instant::now();
    let accepted = loop {
        node.poll(Instant::now()).unwrap();
        if let Some(bytes) = client.poll(4096, 4096).unwrap() {
            break bytes;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
    };
    proof.finish(&accepted).unwrap();
    use std::io::Write;
    for request in [8, 9] {
        let query = g::QueryOperation {
            request_id: Some(g::RequestId {
                value: vec![request; 16],
            }),
            principal: Some(g::Principal {
                account_id: Some(g::AccountId { value: vec![5; 16] }),
                device_id: Some(g::DeviceId { value: vec![6; 16] }),
            }),
            universe_id: Some(g::UniverseId { value: vec![1; 16] }),
            history_id: Some(g::HistoryId { value: vec![2; 16] }),
        };
        let envelope = g::ControlEnvelope {
            protocol_version: 1,
            runtime_session: Some(g::RuntimeSession { value: 9 }),
            required: Some(g::RequiredSemantics {
                capability_ids: vec![1],
                schema_ids: vec![1],
            }),
            body: Some(g::control_envelope::Body::QueryOperation(query)),
            transport: None,
        };
        peer.write_all(&nf_ipc::encode_frame(&envelope.encode_to_vec(), 65_536).unwrap())
            .unwrap();
    }
    let mut replies = 0;
    let start = Instant::now();
    while replies < 2 {
        node.poll(Instant::now()).unwrap();
        if let Some(bytes) = client.poll(4096, 4096).unwrap() {
            assert!(matches!(
                nf_wire::decode_control(&bytes).unwrap().body,
                Some(g::control_envelope::Body::Error(_))
            ));
            replies += 1;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    node.invalidate();
    assert_eq!(node.connection_count(), 0);
    publication.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
