//! Real elapsed initial-call checks. This simulates synchronous delay, not a disk fault.
use super::fixture::Fixture;
use libp2p::{PeerId, swarm::ConnectionId};
use nf_transport::{PeerError, receipt_effects::*};
#[test]
fn elapsed_initial_hello_refuses_without_renewing_request_lifetime() {
    let mut f = Fixture::new();
    let id = ConnectionId::new_unchecked(1);
    let cp = PeerId::from_bytes(&f.client_public.peer).unwrap();
    let mut client = ReceiptClientHandshake::new();
    let mut server = ReceiptServerHandshake::new();
    let hello = client.hello(&mut f.client).unwrap();
    let _delay = ReceiptRepo::delay_current_read_for_test(0);
    assert!(matches!(
        server.begin(hello, cp, id, &mut f.server),
        Err(PeerError::Replay)
    ));
    assert_eq!(server.pending_delivery(), (0, 0, 0));
}
#[test]
fn elapsed_initial_begin_refuses_without_renewing_request_lifetime() {
    let mut f = Fixture::new();
    let id = ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let cp = PeerId::from_bytes(&f.client_public.peer).unwrap();
    let (original, minimum) = f.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let _delay = ReceiptRepo::delay_current_read_for_test(0);
    assert!(matches!(
        server.begin(begin, cp, id, &mut f.server),
        Err(PeerError::Replay)
    ));
    assert_eq!(server.pending_delivery(), (0, 0, 0));
}
