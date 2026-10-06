use nf_contract::canonical::{Limits, Profile, decode_profile};

#[test]
fn bounded_decoder_accepts_the_published_empty_record() {
    let bytes = b"NF-CANON-1\0\xff\0\x01\0\x01\0\0\0\0\0\0\0\0\0\0\0";
    assert_eq!(
        decode_profile(bytes, Limits::default()),
        Ok(Profile::default())
    );
}

#[test]
fn malformed_lengths_presence_headers_and_trailing_bytes_reject() {
    let empty = b"NF-CANON-1\0\xff\0\x01\0\x01\0\0\0\0\0\0\0\0\0\0\0";
    for length in 0..empty.len() {
        assert!(decode_profile(&empty[..length], Limits::default()).is_err());
    }
    let mut invalid = empty.to_vec();
    invalid[17] = 2;
    assert!(decode_profile(&invalid, Limits::default()).is_err());
    invalid = empty.to_vec();
    invalid[15] = 2;
    assert!(decode_profile(&invalid, Limits::default()).is_err());
    invalid = empty.to_vec();
    invalid.push(0);
    assert!(decode_profile(&invalid, Limits::default()).is_err());
    invalid = empty.to_vec();
    invalid[19..23].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_profile(&invalid, Limits::default()),
        Err(nf_contract::canonical::Error::Limit)
    );
    assert_eq!(
        decode_profile(
            empty,
            Limits {
                record_bytes: 26,
                ..Default::default()
            }
        ),
        Err(nf_contract::canonical::Error::Limit)
    );
}

#[test]
fn deterministic_mutation_corpus_never_panics_or_accepts_alternative_encoding() {
    use nf_contract::canonical::encode_profile;
    let seed = encode_profile(
        &Profile {
            optional_text: Some("é".into()),
            ordered_values: vec![i64::MIN, -1, 0, i64::MAX],
            map: vec![("a".into(), u64::MAX)],
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    let limits = Limits {
        record_bytes: 128,
        field_bytes: 8,
        entries: 4,
    };
    let mut state = 0x51a7_u64;
    for _ in 0..20_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut input = seed.clone();
        let index = state as usize % input.len();
        input[index] = (state >> 32) as u8;
        if state & 3 == 0 {
            input.truncate(index);
        }
        if let Ok(decoded) = decode_profile(&input, limits) {
            assert_eq!(encode_profile(&decoded, limits).unwrap(), input);
        }
    }
}

#[test]
fn caller_limits_can_only_lower_profile_hard_caps() {
    use nf_contract::canonical::{Error, encode_profile};
    let empty = b"NF-CANON-1\0\xff\0\x01\0\x01\0\0\0\0\0\0\0\0\0\0\0";
    for limits in [
        Limits {
            entries: 4097,
            ..Default::default()
        },
        Limits {
            field_bytes: 262_145,
            ..Default::default()
        },
        Limits {
            record_bytes: 1_048_577,
            ..Default::default()
        },
    ] {
        assert_eq!(
            encode_profile(&Profile::default(), limits),
            Err(Error::Limit)
        );
        assert_eq!(decode_profile(empty, limits), Err(Error::Limit));
    }
    let restricted = Limits {
        entries: 0,
        field_bytes: 0,
        record_bytes: 27,
    };
    assert_eq!(
        encode_profile(&Profile::default(), restricted).unwrap(),
        empty
    );
    assert_eq!(decode_profile(empty, restricted), Ok(Profile::default()));
    assert_eq!(
        encode_profile(
            &Profile {
                ordered_values: vec![0; 4097],
                ..Default::default()
            },
            Limits {
                entries: 4097,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    );
}
