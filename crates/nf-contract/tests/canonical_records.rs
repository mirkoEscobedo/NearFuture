mod support;
use nf_contract::canonical::records::{decode_record, encode_record, record_digest};

#[test]
fn closed_semantic_records_match_independent_golden_bytes_and_hashes() {
    for vector in support::corpus()["semantic_records"].as_array().unwrap() {
        let bytes = support::bytes(vector["canonical_hex"].as_str().unwrap());
        let record = decode_record(&bytes).unwrap();
        assert_eq!(record.domain as u64, vector["domain"].as_u64().unwrap());
        assert_eq!(record.kind as u64, vector["type"].as_u64().unwrap());
        assert_eq!(encode_record(&record).unwrap(), bytes, "{}", vector["name"]);
        assert_eq!(
            record_digest(&record).unwrap().as_slice(),
            support::bytes(vector["sha256"].as_str().unwrap())
        );
    }
}

#[test]
fn cumulative_payload_entries_and_zero_selectors_reject() {
    use nf_contract::canonical::records::{Record, Value};
    use nf_contract::canonical::{Limits, Profile, encode_profile};
    let corpus = support::corpus();
    let vector = corpus["semantic_records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["domain"] == 3)
        .unwrap();
    let mut proposal =
        decode_record(&support::bytes(vector["canonical_hex"].as_str().unwrap())).unwrap();
    let body = encode_profile(
        &Profile {
            ordered_values: vec![0; 4096],
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    let payload = Record {
        domain: 6,
        kind: 1,
        fields: vec![Value::U32(1), Value::Bytes(body)],
    };
    proposal.fields[8] = Value::Records(vec![payload.clone(); 5]);
    assert!(encode_record(&proposal).is_err());
    // One bounded payload remains valid; the rejection concerns cumulative work.
    proposal.fields[8] = Value::Records(vec![payload]);
    assert!(encode_record(&proposal).is_ok());
    let error = Record {
        domain: 6,
        kind: 10,
        fields: vec![
            Value::U32(1),
            Value::Text(String::new()),
            Value::Set(vec![0]),
            Value::Set(vec![]),
            Value::Bool(false),
        ],
    };
    assert!(encode_record(&error).is_err());
}

#[test]
fn semantic_mutations_never_panic_or_accept_alternative_bytes() {
    let corpus = support::corpus();
    let vectors = corpus["semantic_records"].as_array().unwrap();
    let mut rng = 0x1307_u64;
    for i in 0..20_000 {
        let mut input = support::bytes(
            vectors[i % vectors.len()]["canonical_hex"]
                .as_str()
                .unwrap(),
        );
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let index = rng as usize % input.len();
        input[index] = (rng >> 32) as u8;
        if rng & 3 == 0 {
            input.truncate(index);
        }
        if let Ok(record) = decode_record(&input) {
            assert_eq!(encode_record(&record).unwrap(), input);
        }
    }
}
