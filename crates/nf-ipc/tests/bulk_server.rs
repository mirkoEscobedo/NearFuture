use nf_contract::identity::{AccountId, DeviceId};
use nf_ipc::*;
use nf_wire::generated as g;
use prost::Message;
use sha2::{Digest, Sha256};
use std::{
    net::TcpStream,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
struct Staging {
    bytes: Arc<AtomicUsize>,
    aborts: Arc<AtomicUsize>,
}
impl QueryPort for Staging {
    fn query(&mut self, _: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        Err(IpcError::Unsupported)
    }
    fn stage_chunk(&mut self, piece: TransferPiece) -> Result<(), IpcError> {
        self.bytes.fetch_add(piece.data.len(), Ordering::SeqCst);
        Ok(())
    }
    fn abort_bulk(&mut self) {
        self.bytes.store(0, Ordering::SeqCst);
        self.aborts.fetch_add(1, Ordering::SeqCst);
    }
}
fn config() -> SessionConfig {
    SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits: default_limits(),
    }
}
fn principal() -> LocalPrincipal {
    LocalPrincipal {
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
    }
}
fn authenticate<P: QueryPort>(node: &mut NodeServer<P>, role: EndpointRole) -> FramePump {
    let binding = AuthBinding {
        config: config(),
        principal: principal(),
        role,
        port: node.address().port(),
    };
    node.install_authenticator(
        Authenticator::new(binding.clone(), SecretToken::from_bytes([7; 32])).unwrap(),
    )
    .unwrap();
    let mut client = FramePump::new(TcpStream::connect(node.address()).unwrap(), 4096).unwrap();
    let hello =
        ClientHello::start(binding, SecretToken::from_bytes([7; 32]), default_limits()).unwrap();
    client.send(hello.hello()).unwrap();
    let challenge = receive(node, &mut client);
    let proof = hello.respond(&challenge).unwrap();
    client.send(proof.proof()).unwrap();
    let finished = receive(node, &mut client);
    client.activate(&proof.finish(&finished).unwrap()).unwrap();
    client
}
fn receive<P: QueryPort>(node: &mut NodeServer<P>, peer: &mut FramePump) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        node.poll(Instant::now()).unwrap();
        if let Some(body) = peer.poll(4096, 4096).unwrap() {
            return body;
        }
        assert!(Instant::now() < deadline);
    }
}
#[test]
fn bulk_staging_and_idle_saturation_preserve_separate_control_and_abort_on_disconnect() {
    let bytes = Arc::new(AtomicUsize::new(0));
    let aborts = Arc::new(AtomicUsize::new(0));
    let mut bulk = NodeServer::bind_bulk(
        config(),
        principal(),
        Staging {
            bytes: bytes.clone(),
            aborts: aborts.clone(),
        },
    )
    .unwrap();
    let mut bulk_peer = authenticate(&mut bulk, EndpointRole::Bulk);
    let mut control = NodeServer::bind(config(), principal(), NoOperations).unwrap();
    let mut control_peer = authenticate(&mut control, EndpointRole::Control);
    // A second bulk connection can sit in the OS listen backlog but cannot allocate another worker slot.
    let queued = TcpStream::connect(bulk.address()).unwrap();
    let chunk = g::SnapshotChunk {
        transfer_id: Some(g::OperationId { value: vec![8; 16] }),
        chunk_index: 0,
        chunk_count: 2,
        total_bytes: 3,
        snapshot_digest: Some(g::Sha256Digest {
            value: Sha256::digest(b"abc").to_vec(),
        }),
        data: vec![b'a'],
    }
    .encode_to_vec();
    bulk_peer.send(&chunk).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while bytes.load(Ordering::SeqCst) == 0 {
        bulk_peer.poll(4096, 4096).unwrap();
        bulk.poll(Instant::now()).unwrap();
        assert!(Instant::now() < deadline);
    }
    assert_eq!(bytes.load(Ordering::SeqCst), 1);
    assert_eq!(bulk.connection_count(), 1);
    let query = g::QueryOperation {
        request_id: Some(g::RequestId { value: vec![8; 16] }),
        principal: Some(g::Principal {
            account_id: Some(g::AccountId { value: vec![5; 16] }),
            device_id: Some(g::DeviceId { value: vec![6; 16] }),
        }),
        universe_id: Some(g::UniverseId { value: vec![1; 16] }),
        history_id: Some(g::HistoryId { value: vec![2; 16] }),
    };
    let control_frame = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 9 }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![1],
            schema_ids: vec![1],
        }),
        body: Some(g::control_envelope::Body::QueryOperation(query)),
        transport: None,
    }
    .encode_to_vec();
    control_peer.send(&control_frame).unwrap();
    assert!(matches!(
        nf_wire::decode_control(&receive(&mut control, &mut control_peer))
            .unwrap()
            .body,
        Some(g::control_envelope::Body::Error(_))
    ));
    assert_eq!(bulk.connection_count(), 1);
    drop(bulk_peer);
    let before = aborts.load(Ordering::SeqCst);
    let deadline = Instant::now() + Duration::from_secs(2);
    while aborts.load(Ordering::SeqCst) == before {
        bulk.poll(Instant::now()).unwrap();
        assert!(Instant::now() < deadline);
    }
    assert_eq!(bytes.load(Ordering::SeqCst), 0);
    // Explicit logical-clock advancement exercises inactivity cleanup, not a game timing measurement.
    bulk.poll(Instant::now()).unwrap();
    bulk.poll(Instant::now() + Duration::from_secs(3)).unwrap();
    assert_eq!(bulk.connection_count(), 0);
    drop(queued);
    bulk.invalidate();
    control.invalidate();
}
