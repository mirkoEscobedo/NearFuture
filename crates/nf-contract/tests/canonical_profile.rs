use nf_contract::canonical::{Limits, Profile, encode_profile};

#[test]
fn absent_fields_have_explicit_presence_and_fixed_header() {
    let profile = Profile::default();
    assert_eq!(
        encode_profile(&profile, Limits::default()).unwrap(),
        [
            0x4e, 0x46, 0x2d, 0x43, 0x41, 0x4e, 0x4f, 0x4e, 0x2d, 0x31, 0, 255, 0, 1, 0, 1, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn present_empty_is_distinct_from_absent() {
    let profile = Profile {
        optional_bytes: Some(vec![]),
        optional_text: Some(String::new()),
        ..Default::default()
    };
    let encoded = encode_profile(&profile, Limits::default()).unwrap();
    assert_eq!(
        &encoded[17..],
        &[1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn decomposed_schema_text_rejects_without_normalizing_identity() {
    let profile = Profile {
        optional_text: Some("e\u{301}".into()),
        ..Default::default()
    };
    assert_eq!(
        encode_profile(&profile, Limits::default()),
        Err(nf_contract::canonical::Error::InvalidProfile)
    );
}

#[test]
fn maps_reject_duplicates_and_unordered_utf8_keys() {
    for map in [
        vec![("a".into(), 1), ("a".into(), 2)],
        vec![("z".into(), 1), ("a".into(), 2)],
    ] {
        assert_eq!(
            encode_profile(
                &Profile {
                    map,
                    ..Default::default()
                },
                Limits::default()
            ),
            Err(nf_contract::canonical::Error::InvalidProfile)
        );
    }
}

#[test]
fn text_rejects_newer_unicode_and_noncharacters_under_the_shared_profile() {
    for text in ["\u{105c9}", "\u{1fae0}", "\u{ffff}"] {
        let value = Profile {
            optional_text: Some(text.into()),
            ..Default::default()
        };
        assert_eq!(
            encode_profile(&value, Limits::default()),
            Err(nf_contract::canonical::Error::InvalidProfile)
        );
    }
    assert!(
        encode_profile(
            &Profile {
                optional_text: Some("é🚀".into()),
                ..Default::default()
            },
            Limits::default()
        )
        .is_ok()
    );
}
