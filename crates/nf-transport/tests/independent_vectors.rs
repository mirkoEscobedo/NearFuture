use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::{
    auth::{BulkTranscript, HandshakeContext, QueryTranscript},
    records::*,
};
use std::collections::BTreeMap;
fn hex(s: &str) -> Vec<u8> {
    assert!(s.len() <= 20000 && s.len().is_multiple_of(2));
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn rows() -> BTreeMap<&'static str, Vec<&'static str>> {
    include_str!("../../../docs/transport/vectors/peer-v1.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let f: Vec<_> = l.split('\t').collect();
            assert_eq!(f.len(), 6);
            (f[1], f)
        })
        .collect()
}
#[test]
fn independent_node_raw_corpus_and_real_ed25519_proof_signatures_match_closed_admission() {
    let rows = rows();
    let mut count = 0;
    for row in rows.values().filter(|r| r[0] == "raw") {
        let lane = if row[2] == "1" {
            Lane::Control
        } else {
            Lane::Bulk
        };
        let bytes = hex(row[4]);
        match decode_body(&bytes, lane, PeerLimits::default()) {
            Ok(r) => {
                assert_eq!(row[3], "OK", "{}", row[1]);
                assert_eq!(encode_body(&r, lane, PeerLimits::default()).unwrap(), bytes);
                let proof = match &r.body {
                    PeerBody::ServerHello { proof, .. }
                    | PeerBody::Finished(proof)
                    | PeerBody::RetainedStatus { proof, .. }
                    | PeerBody::Unsupported { proof, .. }
                    | PeerBody::BulkReady { proof, .. }
                    | PeerBody::BulkProgress { proof, .. }
                    | PeerBody::BulkVerified { proof, .. } => Some((proof, "server-device-key")),
                    PeerBody::ClientProof(proof)
                    | PeerBody::ProveQuery { proof, .. }
                    | PeerBody::ProveBulk { proof, .. } => Some((proof, "client-device-key")),
                    _ => None,
                };
                if let Some((p, k)) = proof {
                    let key: [u8; 32] = hex(rows[k][4]).try_into().unwrap();
                    let digest = nf_identity::signing::device_digest(p).unwrap();
                    nf_contract::signatures::verify_digest(&key, &digest, &p.signature).unwrap();
                    let mut changed = p.signature;
                    changed[0] ^= 1;
                    assert!(
                        nf_contract::signatures::verify_digest(&key, &digest, &changed).is_err()
                    );
                }
                count += 1;
            }
            Err(e) => assert_eq!(format!("{e:?}"), row[3], "{}", row[1]),
        }
    }
    assert_eq!(count, 20);
}
#[test]
fn independent_node_handshake_query_bulk_and_each_reply_prefix_transcript_match() {
    let rows = rows();
    for lane in [Lane::Control, Lane::Bulk] {
        let h = HandshakeContext {
            lane,
            client_peer: libp2p::PeerId::from_bytes(&hex(rows["client-peer"][4])).unwrap(),
            server_peer: libp2p::PeerId::from_bytes(&hex(rows["server-peer"][4])).unwrap(),
            client_account: AccountId::from_bytes([1; 16]),
            client_device: DeviceId::from_bytes([2; 16]),
            server_account: AccountId::from_bytes([3; 16]),
            server_device: DeviceId::from_bytes([4; 16]),
            context: PeerContext {
                session: [5; 16],
                scope: Scope {
                    universe: UniverseId::from_bytes([6; 16]),
                    history: HistoryId::from_bytes([7; 16]),
                },
                ruleset: [8; 32],
                content: [9; 32],
            },
            client_nonce: [10; 32],
            server_nonce: [11; 32],
            required: lane as u32,
            optional: if lane == Lane::Control { 2 } else { 1 },
            server_available: 3,
            selected_caps: 3,
            offered: PeerLimits::default(),
            server_limits: PeerLimits::default(),
            selected: PeerLimits::default(),
        };
        for stage in 0..=3 {
            let expected = hex(rows[format!("auth-{}-{stage}", lane as u8).as_str()][5]);
            let digest = if stage == 0 {
                h.context_digest().unwrap()
            } else {
                h.challenge(stage, 12).unwrap()
            };
            assert_eq!(digest.as_slice(), expected);
        }
        let t = QueryTranscript {
            context_digest: h.context_digest().unwrap(),
            request: RequestId::from_bytes([13; 16]),
            client_nonce: [14; 32],
            server_nonce: [15; 32],
            frontier: 12,
            minimum_membership: 11,
        };
        if lane == Lane::Control {
            assert_eq!(
                t.challenge(1, [0; 32]).unwrap().as_slice(),
                hex(rows["op-1-1"][5])
            );
        } else {
            let descriptor = BulkDescriptor {
                transfer: [13; 16],
                total: 6,
                chunks: 2,
                digest: sha2::Sha256::digest(b"abcdef").into(),
            };
            let t = BulkTranscript {
                context_digest: h.context_digest().unwrap(),
                descriptor,
                client_nonce: [14; 32],
                server_nonce: [15; 32],
                frontier: 12,
                minimum_membership: 11,
            };
            assert_eq!(
                t.challenge(1, [0; 32]).unwrap().as_slice(),
                hex(rows["op-2-1"][5])
            );
            for name in ["bulk-ready", "bulk-progress", "bulk-verified"] {
                let raw = hex(rows[name][4]);
                let r = decode_body(&raw, Lane::Bulk, PeerLimits::default()).unwrap();
                let prefix = reply_prefix_digest(&r, Lane::Bulk, PeerLimits::default()).unwrap();
                let digest = t.challenge(2, prefix).unwrap();
                assert_eq!(
                    digest.as_slice(),
                    hex(rows[format!("{name}-reply").as_str()][5])
                );
                let p = match r.body {
                    PeerBody::BulkReady { proof, .. }
                    | PeerBody::BulkProgress { proof, .. }
                    | PeerBody::BulkVerified { proof, .. } => proof,
                    _ => panic!(),
                };
                assert_eq!(p.challenge, digest);
            }
        }
    }
}
use sha2::Digest;
