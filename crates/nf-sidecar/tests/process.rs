//! Actual bounded child/socket observations. Every child and reader has an owner.
use nf_modules::Invocation;
#[cfg(feature = "snapshot")]
use nf_modules::{Context, Host};
use nf_sidecar::protocol::{self, Channel, Limits, Offer};
#[cfg(feature = "snapshot")]
use std::sync::atomic::AtomicBool;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

struct Owned {
    child: Child,
    output: mpsc::Receiver<Vec<u8>>,
    reader: Option<JoinHandle<()>>,
    token: protocol::Token,
    session: [u8; 16],
    address: SocketAddr,
}
impl Owned {
    fn start(capabilities: u32, lifetime: u32) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nf-sidecar"))
            .arg("serve-parent")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let mut first = vec![0; 34];
            if stdout.read_exact(&mut first).is_err() || tx.send(first).is_err() {
                return;
            }
            loop {
                let mut ack = vec![0; 32];
                if stdout.read_exact(&mut ack).is_err() || tx.send(ack).is_err() {
                    break;
                }
            }
        });
        let mut owned = Self {
            child,
            output: rx,
            reader: Some(reader),
            token: Zeroizing::new([9; 32]),
            session: [0; 16],
            address: "127.0.0.1:1".parse().unwrap(),
        };
        let mut b = Vec::new();
        b.extend_from_slice(b"NFMPAR01");
        b.extend_from_slice(owned.token.as_ref());
        b.extend_from_slice(&lifetime.to_be_bytes());
        b.extend_from_slice(&capabilities.to_be_bytes());
        owned.child.stdin.as_mut().unwrap().write_all(&b).unwrap();
        let ready = owned
            .output
            .recv_timeout(Duration::from_secs(3))
            .expect("bounded child READY");
        assert_eq!(&ready[..8], b"NFMRDY01");
        owned
            .address
            .set_port(u16::from_be_bytes(ready[8..10].try_into().unwrap()));
        owned.session.copy_from_slice(&ready[10..26]);
        assert_ne!(owned.session, [0; 16]);
        owned
    }
    fn connect(&self, offer: Offer) -> (TcpStream, Channel) {
        let mut socket = TcpStream::connect_timeout(&self.address, Duration::from_secs(1)).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let h = protocol::hello(
            &self.token,
            self.session,
            self.address.port(),
            [3; 16],
            &offer,
        );
        write_frame(&mut socket, &h);
        let a = protocol::finish(&self.token, self.session, &h, &read_frame(&mut socket)).unwrap();
        assert_eq!(a.status, 0);
        (socket, a.channel)
    }
    fn request<'a>(&self, module: u16, payload: &'a [u8]) -> Invocation<'a> {
        Invocation {
            session: self.session,
            request_id: [2; 16],
            campaign_generation: 0,
            module,
            operation: 1,
            api_version: 1,
            required_capabilities: if module == 2 { 2 } else { 1 },
            payload,
        }
    }
    fn bind_fragmented(&mut self, generation: u64) {
        let mut command = vec![1];
        command.extend_from_slice(&generation.to_be_bytes());
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&command[..3])
            .unwrap();
        assert!(self.output.recv_timeout(Duration::from_millis(15)).is_err());
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&command[3..])
            .unwrap();
        let ack = self.output.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(&ack[..8], b"NFMBND01");
        assert_eq!(&ack[8..24], &self.session);
        assert_eq!(
            u64::from_be_bytes(ack[24..].try_into().unwrap()),
            generation
        );
    }
    fn await_exit(&mut self) {
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(Instant::now() < end, "child exceeded bounded shutdown");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.child.stdin.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        // Release a potentially queued ACK before joining the pipe reader.
        let (_, replacement) = mpsc::sync_channel(1);
        drop(std::mem::replace(&mut self.output, replacement));
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
fn write_frame(s: &mut TcpStream, b: &[u8]) {
    s.write_all(&(b.len() as u32).to_be_bytes()).unwrap();
    s.write_all(b).unwrap();
}
fn read_frame(s: &mut TcpStream) -> Vec<u8> {
    let mut n = [0; 4];
    s.read_exact(&mut n).unwrap();
    let size = u32::from_be_bytes(n) as usize;
    assert!((1..=protocol::MAX_FRAME).contains(&size));
    let mut b = vec![0; size];
    s.read_exact(&mut b).unwrap();
    b
}
fn closed(s: &mut TcpStream) {
    match s.read(&mut [0; 1]) {
        Ok(0) => {}
        Err(e) => assert!(matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
        )),
        other => panic!("expected closed, got {other:?}"),
    }
}

#[test]
fn real_process_ack_order_fragment_eof_and_campaign_fence() {
    let mut owner = Owned::start(1, 5000);
    let (mut socket, ch) = owner.connect(Offer::default());
    let request = owner.request(1, &[7; 32]);
    write_frame(&mut socket, &ch.encode_request(&request, 1, 1000).unwrap());
    assert_eq!(
        ch.decode_reply(&read_frame(&mut socket), &request, 1)
            .unwrap()
            .outcome,
        Ok(vec![7; 32])
    );
    owner.bind_fragmented(1);
    closed(&mut socket);
    let (mut new_socket, ch) = owner.connect(Offer::default());
    let mut request = owner.request(1, &[8; 32]);
    request.campaign_generation = 1;
    write_frame(
        &mut new_socket,
        &ch.encode_request(&request, 1, 1000).unwrap(),
    );
    assert_eq!(
        ch.decode_reply(&read_frame(&mut new_socket), &request, 1)
            .unwrap()
            .outcome,
        Ok(vec![8; 32])
    );
    // EOF halfway through a following command revokes, rather than acknowledging a partial generation.
    owner
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&[1, 0, 0])
        .unwrap();
    owner.child.stdin.take();
    owner.await_exit();
    closed(&mut new_socket);
}
#[test]
fn real_process_sequence_authentication_negotiated_frame_and_receive_deadline() {
    let owner = Owned::start(1, 5000);
    let (mut socket, ch) = owner.connect(Offer::default());
    let request = owner.request(1, &[7; 32]);
    let b = ch.encode_request(&request, 1, 1000).unwrap();
    write_frame(&mut socket, &b);
    assert!(
        ch.decode_reply(&read_frame(&mut socket), &request, 1)
            .unwrap()
            .outcome
            .is_ok()
    );
    write_frame(&mut socket, &b);
    closed(&mut socket);
    let (mut socket, ch) = owner.connect(Offer::default());
    let mut b = ch.encode_request(&request, 1, 1000).unwrap();
    b[72] ^= 1;
    write_frame(&mut socket, &b);
    closed(&mut socket);
    let (mut socket, _) = owner.connect(Offer {
        limits: Limits {
            request_bytes: 136,
            ..Limits::default()
        },
        ..Offer::default()
    });
    // No body sent: selected cap must reject this header immediately, before allocating/waiting.
    socket.write_all(&137u32.to_be_bytes()).unwrap();
    closed(&mut socket);
    let (mut socket, _) = owner.connect(Offer {
        limits: Limits {
            timeout_ms: 10,
            ..Limits::default()
        },
        ..Offer::default()
    });
    std::thread::sleep(Duration::from_millis(30));
    closed(&mut socket);
}
#[test]
fn finite_service_expiry_and_bounded_generation_commands() {
    let mut expires = Owned::start(1, 250);
    expires.await_exit();
    let mut owner = Owned::start(1, 5000);
    for generation in 1..=8 {
        owner.bind_fragmented(generation);
    }
    owner
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&[1, 0, 0, 0, 0, 0, 0, 0, 9])
        .unwrap();
    owner.await_exit();
}

#[cfg(feature = "snapshot")]
#[test]
fn same_canonical_inspection_headlessly_and_in_actual_sidecar() {
    use nf_contract::identity::{AggregateId, AggregateRevision, EntityId, HistoryId, UniverseId};
    let mut spec = nf_kernel::WorldSpec::empty(
        UniverseId::from_bytes([1; 16]),
        HistoryId::from_bytes([2; 16]),
        [3; 32],
        [4; 32],
    );
    for id in [10, 11] {
        spec.factions.push(nf_kernel::Faction {
            id: EntityId::from_bytes([id; 16]),
            aggregate: AggregateId::from_bytes([id; 16]),
        });
        spec.revisions
            .insert(AggregateId::from_bytes([id; 16]), AggregateRevision(0));
    }
    let world = nf_kernel::World::new(spec).unwrap();
    let bytes = nf_kernel::encode_snapshot(&world).unwrap();
    let expected = nf_kernel::state_hash(&world).unwrap();
    let owner = Owned::start(3, 5000);
    let (mut socket, ch) = owner.connect(Offer {
        requested: 3,
        required: 3,
        ..Offer::default()
    });
    let host = Host::new(owner.session, true)
        .unwrap()
        .with_snapshot(true)
        .unwrap();
    let cancelled = AtomicBool::new(false);
    let context = Context {
        deadline: Instant::now() + Duration::from_secs(1),
        cancelled: &cancelled,
    };
    let request = owner.request(2, &bytes);
    let local = host.dispatch(&request, 3, &context);
    write_frame(&mut socket, &ch.encode_request(&request, 1, 1000).unwrap());
    let remote = ch
        .decode_reply(&read_frame(&mut socket), &request, 1)
        .unwrap();
    assert_eq!(remote, local);
    let payload = remote.outcome.unwrap();
    assert_eq!(&payload[..2], &1u16.to_be_bytes());
    assert_eq!(&payload[2..], &expected);
    let mut trailing = bytes.clone();
    trailing.push(0);
    for (index, malformed) in [
        &bytes[..bytes.len() - 1],
        &b"malformed"[..],
        trailing.as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        let r = owner.request(2, malformed);
        let local = host.dispatch(&r, 3, &context);
        assert_eq!(local.outcome, Err(nf_modules::Error::InvalidSnapshot));
        let sequence = index as u64 + 2;
        write_frame(&mut socket, &ch.encode_request(&r, sequence, 1000).unwrap());
        assert_eq!(
            ch.decode_reply(&read_frame(&mut socket), &r, sequence)
                .unwrap(),
            local
        );
    }
    // Expected handler rejection leaves the independent probe available.
    let r = owner.request(1, &[5; 32]);
    write_frame(&mut socket, &ch.encode_request(&r, 5, 1000).unwrap());
    assert_eq!(
        ch.decode_reply(&read_frame(&mut socket), &r, 5)
            .unwrap()
            .outcome,
        Ok(vec![5; 32])
    );
    let oversized = vec![0; nf_modules::MAX_PAYLOAD + 1];
    assert_eq!(
        host.dispatch(&owner.request(2, &oversized), 3, &context)
            .outcome,
        Err(nf_modules::Error::Limit)
    );
    let (mut small, small_channel) = owner.connect(Offer {
        requested: 3,
        required: 3,
        limits: Limits {
            response_bytes: 122,
            ..Limits::default()
        },
        ..Offer::default()
    });
    write_frame(
        &mut small,
        &small_channel.encode_request(&request, 1, 1000).unwrap(),
    );
    assert_eq!(
        small_channel
            .decode_reply(&read_frame(&mut small), &request, 1)
            .unwrap()
            .outcome,
        Err(nf_modules::Error::Limit)
    );
    let r = owner.request(1, &[6; 32]);
    write_frame(
        &mut small,
        &small_channel.encode_request(&r, 2, 1000).unwrap(),
    );
    assert_eq!(
        small_channel
            .decode_reply(&read_frame(&mut small), &r, 2)
            .unwrap()
            .outcome,
        Ok(vec![6; 32])
    );
}
