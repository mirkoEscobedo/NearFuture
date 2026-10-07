use nf_ipc::{FramePump, IpcError, LoopbackListener};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, Instant};
#[test]
fn real_loopback_handles_partial_peer_slow_peer_and_eof() {
    let listener = LoopbackListener::bind().unwrap();
    let peer = TcpStream::connect(listener.address()).unwrap();
    let start = Instant::now();
    let server = loop {
        if let Some(v) = listener.try_accept(32).unwrap() {
            break v;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::yield_now();
    };
    let mut server = server;
    let mut client = FramePump::new(peer.try_clone().unwrap(), 32).unwrap();
    for _ in 0..100 {
        assert_eq!(server.poll(4, 4).unwrap(), None);
    }
    client.send(&[7, 8, 9]).unwrap();
    assert_eq!(client.send(&[1]), Err(IpcError::Backpressure));
    let mut received = None;
    let start = Instant::now();
    while received.is_none() {
        client.poll(1, 1).unwrap();
        received = server.poll(1, 1).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    assert_eq!(received, Some(vec![7, 8, 9]));
    peer.shutdown(Shutdown::Write).unwrap();
    let start = Instant::now();
    loop {
        match server.poll(4, 4) {
            Err(IpcError::Disconnected) => break,
            Ok(None) => {}
            other => panic!("Unexpected EOF: {other:?}"),
        };
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
