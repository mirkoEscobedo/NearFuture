mod support;
use nf_contract::canonical::{Limits, Profile, decode_profile, encode_profile};
use sha2::{Digest, Sha256};

#[test]
fn all_published_positive_bytes_and_sha256_hashes_match() {
    for vector in support::corpus()["positive"].as_array().unwrap() {
        let record = &vector["record"];
        let profile = Profile {
            optional_bytes: record["optional_bytes_hex"].as_str().map(support::bytes),
            optional_text: record["optional_text"].as_str().map(String::from),
            ordered_values: record["integers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_str().unwrap().parse().unwrap())
                .collect(),
            map: record["counters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| {
                    (
                        entry["key"].as_str().unwrap().into(),
                        entry["value"].as_str().unwrap().parse().unwrap(),
                    )
                })
                .collect(),
        };
        let expected = support::bytes(vector["canonical_hex"].as_str().unwrap());
        let encoded = encode_profile(&profile, Limits::default()).unwrap();
        assert_eq!(encoded, expected, "{}", vector["name"]);
        assert_eq!(
            Sha256::digest(&encoded).as_slice(),
            support::bytes(vector["sha256"].as_str().unwrap())
        );
        assert_eq!(
            decode_profile(&expected, Limits::default()).unwrap(),
            profile
        );
    }
}
#[test]
fn every_published_malformed_fixture_rejects() {
    for vector in support::corpus()["malformed"].as_array().unwrap() {
        let input = support::bytes(vector["canonical_hex"].as_str().unwrap());
        assert!(
            decode_profile(&input, Limits::default()).is_err(),
            "{}",
            vector["name"]
        );
    }
}
