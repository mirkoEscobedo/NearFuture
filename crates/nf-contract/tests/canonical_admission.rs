use nf_contract::canonical::records::{Record, Value, encode_record};
use nf_contract::canonical::{Error, Limits, Profile, encode_profile};

fn payload() -> Record {
    Record {
        domain: 6,
        kind: 1,
        fields: vec![
            Value::U32(1),
            Value::Bytes(encode_profile(&Profile::default(), Limits::default()).unwrap()),
        ],
    }
}
fn status() -> Record {
    Record {
        domain: 6,
        kind: 8,
        fields: vec![
            Value::Id([1; 16]),
            Value::Id([2; 16]),
            Value::Id([3; 16]),
            Value::Digest([4; 32]),
            Value::U32(1),
            Value::Outcome(None),
            Value::OptionalU64(None),
        ],
    }
}
#[test]
fn dedup_entries_cannot_rebind_a_status_to_another_request() {
    let entry = Record {
        domain: 6,
        kind: 9,
        fields: vec![
            Value::Id([5; 16]),
            Value::Digest([4; 32]),
            Value::Record(Box::new(status())),
        ],
    };
    assert_eq!(encode_record(&entry), Err(Error::InvalidProfile));
}
#[test]
fn succeeded_status_requires_payload_and_committed_frontier() {
    let mut status = status();
    status.fields[4] = Value::U32(3);
    status.fields[5] = Value::Outcome(Some(Box::new(payload())));
    assert!(encode_record(&status).is_err());
    status.fields[6] = Value::OptionalU64(Some(0));
    assert!(encode_record(&status).is_ok());
}

mod support;
#[test]
fn child_operation_histories_must_match_snapshot_and_batch() {
    use nf_contract::canonical::records::decode_record;
    let corpus = support::corpus();
    for (domain, field_index, dedup) in [(2, 10, false), (2, 11, true), (4, 6, false)] {
        let vector = corpus["semantic_records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["domain"] == domain)
            .unwrap();
        let mut outer =
            decode_record(&support::bytes(vector["canonical_hex"].as_str().unwrap())).unwrap();
        let history = outer.fields[if domain == 2 { 1 } else { 0 }].clone();
        let mut child = status();
        child.fields[2] = history;
        let nested = if dedup {
            Record {
                domain: 6,
                kind: 9,
                fields: vec![
                    child.fields[0].clone(),
                    child.fields[3].clone(),
                    Value::Record(Box::new(child)),
                ],
            }
        } else {
            child
        };
        outer.fields[field_index] = Value::Records(vec![nested]);
        let valid = encode_record(&outer).unwrap();
        let child_header = b"NF-CANON-1\0\x06\0\x08\0\x01\0";
        let offset = valid
            .windows(child_header.len())
            .position(|bytes| bytes == child_header)
            .unwrap()
            + child_header.len()
            + 32;
        let mut bad = valid;
        bad[offset] ^= 1;
        assert_eq!(decode_record(&bad), Err(Error::InvalidProfile));
        if let Value::Records(children) = &mut outer.fields[field_index] {
            let status = if dedup {
                match &mut children[0].fields[2] {
                    Value::Record(status) => status,
                    _ => unreachable!(),
                }
            } else {
                &mut children[0]
            };
            status.fields[2] = Value::Id([99; 16]);
        }
        assert_eq!(encode_record(&outer), Err(Error::InvalidProfile));
    }
}
