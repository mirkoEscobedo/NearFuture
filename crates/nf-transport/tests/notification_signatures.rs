mod notification_support;
use nf_contract::signatures::verify_digest;
use nf_identity::{model::DeviceProof, signing::device_digest};
use nf_transport::notification::{NotifyBody, NotifyLimits, PROTOCOL, decode_body};

fn proof(body: &NotifyBody) -> &DeviceProof {
    match body {
        NotifyBody::ServerHello { proof, .. }
        | NotifyBody::ProveSubscribe { proof, .. }
        | NotifyBody::Subscribed { proof, .. }
        | NotifyBody::Notice { proof, .. }
        | NotifyBody::NoticeAck { proof, .. }
        | NotifyBody::ClientProof(proof)
        | NotifyBody::Finished(proof) => proof,
        _ => panic!("fixture has no primitive proof"),
    }
}
#[test]
fn independent_primitive_signatures_verify_and_tamper_rejects_without_policy_claim() {
    let rows = notification_support::vectors();
    let mut count = 0;
    for row in rows.iter().filter(|row| row.category == "signature") {
        assert_eq!(row.layer, "primitive");
        assert_eq!(row.expectation, "VALID_ED25519_DIGEST");
        let name = if row.name == "minimum-limits" {
            "server-hello-minimum-limits"
        } else {
            row.name
        };
        let raw = &rows
            .iter()
            .find(|record| record.category == "record" && record.name == name)
            .unwrap()
            .bytes;
        let record = decode_body(raw, PROTOCOL, NotifyLimits::default()).unwrap();
        let p = proof(&record.body);
        assert_eq!(p.signature.as_slice(), row.bytes);
        // Frozen fixture PeerIds contain the known public Ed25519 keys, not a SQL grant.
        let key: [u8; 32] = p.peer[6..].try_into().unwrap();
        let digest = device_digest(p).unwrap();
        verify_digest(&key, &digest, &p.signature).unwrap();
        let mut tampered = p.signature;
        tampered[63] ^= 1;
        assert!(verify_digest(&key, &digest, &tampered).is_err());
        count += 1;
    }
    assert_eq!(count, 12);
}
