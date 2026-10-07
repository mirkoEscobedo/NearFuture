use super::fixture::Fixture;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    PeerError,
    receipt::*,
    receipt_effects::{operation::VerifiedReceipt, *},
};
fn verified(f: &mut Fixture, below: bool) -> VerifiedReceipt {
    let id = ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, mut minimum) = f.client.original_for_slot(0).unwrap();
    if below {
        minimum.event = nf_contract::identity::EventSeq(2);
    }
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
    let response = server.prove(proof, cp, id, &mut f.server).unwrap();
    client.reply(response, sp, id, &mut f.client).unwrap()
}
#[test]
fn after_read_unsupported_expiry_returns_no_fresh_result() {
    let mut f = Fixture::new();
    let v = verified(&mut f, true);
    let _delay = ReceiptRepo::delay_current_read_for_test(0);
    assert!(matches!(
        f.client.accept_verified(0, v),
        Err(PeerError::Replay)
    ));
    assert_eq!(f.client.book_anchors()[0].generation, 0);
}
#[test]
fn after_read_persisted_status_expiry_retains_only_observational_head() {
    let mut f = Fixture::new();
    let v = verified(&mut f, false);
    let _delay = ReceiptRepo::delay_current_read_for_test(1);
    assert!(matches!(
        f.client.accept_verified(0, v),
        Err(PeerError::Replay)
    ));
    assert_eq!(f.client.book_anchors()[0].generation, 1);
    let head = f
        .client
        .recover_observations()
        .unwrap()
        .head(0)
        .unwrap()
        .clone();
    assert_eq!(head.phase, book::BookPhase::Receipt(ReceiptPhase::Pending));
}
#[tokio::test]
async fn after_read_valid_outbound_expiry_refuses_before_enqueue() {
    let mut f = Fixture::new();
    let id = ConnectionId::new_unchecked(1);
    let (cs, _) = f.sessions(id);
    let (original, minimum) = f.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let mut swarm = f.client.build_receipt_lane().unwrap();
    let _delay = ReceiptRepo::delay_current_read_for_test(0);
    assert!(matches!(
        client.queue_request(begin, sp, id, &mut swarm, &mut f.client),
        Err(PeerError::Replay)
    ));
    assert_eq!(client.pending_delivery(), (0, 0, 0));
    assert_eq!(f.client.book_anchors()[0].generation, 0);
}
