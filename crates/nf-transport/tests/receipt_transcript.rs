#[path = "receipt_support/corpus.rs"]
mod corpus;
use nf_transport::{
    auth::HandshakeContext,
    receipt::{ReceiptBody, ReceiptHandshake, decode_body},
    records::{Lane, PeerLimits},
};
use sha2::{Digest, Sha256};
fn independent_context() -> HandshakeContext {
    assert_eq!(corpus::matching("record", "shape").len(), 14);
    let client = decode_body(&corpus::row("record", "hello"), PeerLimits::default()).unwrap();
    let server = decode_body(
        &corpus::row("record", "server-hello"),
        PeerLimits::default(),
    )
    .unwrap();
    let ReceiptBody::Hello {
        account,
        device,
        nonce,
        required,
        optional,
        offered,
    } = client.body
    else {
        panic!("hello")
    };
    let ReceiptBody::ServerHello {
        nonce: server_nonce,
        available,
        selected_caps,
        server_limits,
        selected,
        proof,
    } = server.body
    else {
        panic!("server hello")
    };
    HandshakeContext {
        lane: Lane::Control,
        client_peer: libp2p::PeerId::from_bytes(&corpus::row("value", "client-peer")).unwrap(),
        server_peer: libp2p::PeerId::from_bytes(&proof.peer).unwrap(),
        client_account: account,
        client_device: device,
        server_account: proof.account,
        server_device: proof.device,
        context: server.context,
        client_nonce: nonce,
        server_nonce,
        required,
        optional,
        server_available: available,
        selected_caps,
        offered,
        server_limits,
        selected,
    }
}
#[test]
fn receipt_authentication_has_distinct_signed_namespace() {
    let context = independent_context();
    let receipt = ReceiptHandshake::new(context.clone()).unwrap();
    let expected: [u8; 32] = Sha256::digest(corpus::row("transcript", "handshake-1")).into();
    assert_eq!(
        receipt.challenge(1, 12).unwrap(),
        expected,
        "receipt/control2 cannot reuse control1 challenge domain"
    );
    assert_ne!(
        receipt.challenge(1, 12).unwrap(),
        context.challenge(1, 12).unwrap()
    );
}
#[test]
fn independent_fixed_transcripts_and_every_signed_result_prefix_match() {
    use nf_transport::receipt::{ReceiptOperation, reply_prefix_digest};
    let h = ReceiptHandshake::new(independent_context()).unwrap();
    for stage in 0..=3 {
        let frontier = if stage == 0 { 0 } else { 12 };
        assert_eq!(
            h.transcript(stage, frontier).unwrap().to_vec(),
            corpus::row("transcript", &format!("handshake-{stage}"))
        );
    }
    let ReceiptBody::Begin {
        target,
        nonce,
        minimum,
    } = decode_body(
        &corpus::row("record", "begin-receipt"),
        PeerLimits::default(),
    )
    .unwrap()
    .body
    else {
        panic!("begin")
    };
    let ReceiptBody::Challenge {
        server_nonce,
        frontier,
        ..
    } = decode_body(
        &corpus::row("record", "receipt-challenge"),
        PeerLimits::default(),
    )
    .unwrap()
    .body
    else {
        panic!("challenge")
    };
    let op = ReceiptOperation {
        context_digest: h.context_digest().unwrap(),
        target,
        client_nonce: nonce,
        server_nonce,
        frontier,
        minimum,
    };
    assert_eq!(
        op.transcript(1, [0; 32]).unwrap().to_vec(),
        corpus::row("transcript", "operation-1")
    );
    for (record_name, transcript_name) in [
        ("status-unknown", "reply-unknown"),
        ("status-pending", "reply-pending"),
        ("status-rejected", "reply-rejected"),
        ("status-committed", "reply-committed"),
        ("binding-conflict", "binding-conflict"),
        ("source-below-minima", "source-below-minima"),
    ] {
        let raw = corpus::row("record", record_name);
        let r = decode_body(&raw, PeerLimits::default()).unwrap();
        assert_eq!(raw[..raw.len() - 297], corpus::row("prefix", record_name));
        let digest = reply_prefix_digest(&r, PeerLimits::default()).unwrap();
        assert_eq!(
            op.transcript(2, digest).unwrap().to_vec(),
            corpus::row("transcript", transcript_name)
        );
    }
    let mut normalized = op;
    normalized.minimum.membership_revision += 1;
    assert_ne!(
        normalized.challenge(1, [0; 32]).unwrap(),
        op.challenge(1, [0; 32]).unwrap()
    );
    assert!(op.transcript(1, [1; 32]).is_err());
    assert!(op.transcript(3, [0; 32]).is_err());
}
