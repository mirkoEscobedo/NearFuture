#[path = "receipt_support/corpus.rs"]
mod corpus;
use nf_identity::signing::device_digest;
use nf_transport::{
    receipt::{ReceiptBody, decode_body},
    records::PeerLimits,
};
#[test]
fn eleven_independent_public_test_signatures_verify_as_primitives_only() {
    assert_eq!(corpus::matching("signature", "primitive").len(), 11);
    for (record_name, signature_name, client) in [
        ("prove-receipt", "operation-client", true),
        ("server-hello", "server-hello", false),
        ("client-proof", "client-proof", true),
        ("finished", "finished", false),
        ("status-unknown", "reply-unknown", false),
        ("status-pending", "reply-pending", false),
        ("status-rejected", "reply-rejected", false),
        ("status-committed", "reply-committed", false),
        ("binding-conflict", "binding-conflict", false),
        ("source-below-minima", "source-below-minima", false),
        ("status-maximum-counters", "reply-maximum-counters", false),
    ] {
        let record =
            decode_body(&corpus::row("record", record_name), PeerLimits::default()).unwrap();
        let proof = match record.body {
            ReceiptBody::Prove { proof, .. }
            | ReceiptBody::ServerHello { proof, .. }
            | ReceiptBody::Status { proof, .. }
            | ReceiptBody::Unsupported { proof, .. }
            | ReceiptBody::ClientProof(proof)
            | ReceiptBody::Finished(proof) => proof,
            _ => panic!("proof fixture"),
        };
        assert_eq!(
            proof.signature.to_vec(),
            corpus::row("signature", signature_name)
        );
        let peer = corpus::row("value", if client { "client-peer" } else { "server-peer" });
        // This independent corpus uses identity-multihash Ed25519 public peers.
        assert_eq!(&peer[..6], &[0, 36, 8, 1, 18, 32]);
        let key: [u8; 32] = peer[6..].try_into().unwrap();
        assert!(
            nf_contract::signatures::verify_digest(
                &key,
                &device_digest(&proof).unwrap(),
                &proof.signature
            )
            .is_ok(),
            "primitive {signature_name}"
        );
    }
    // No admitted MembershipState/actual Swarm session is constructed from these data fixtures.
}
