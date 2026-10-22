use nf_contract::identity::EntityId;
use nf_nex_boundary::Observation;
use nf_nex_shadow::{
    BindingScope, MakePeaceEligibility, ShadowAuthority, ShadowOutput, Unavailable,
    decode_action_input, encode_action_input, replay_synthetic_action,
};

// The externally frozen record has source bytes22..62, digests62..318,
// IDs318..398, subject length398..402 and subject402..410. Its closed tail
// is concern410, runtime411..419, frontier419..427, observation427,
// authority428, kind429, facts430..434. These offsets are fixture assertions.
fn record() -> Vec<u8> {
    let fields: Vec<_> = include_str!("../fixtures/action-identity-v2.tsv")
        .lines()
        .nth(1)
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(fields[0], "no-target");
    let bytes: Vec<u8> = fields[1]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), 434);
    assert_eq!(&bytes[398..410], b"\x08\0\0\0hegemony");
    assert_eq!(&bytes[427..434], &[1, 1, 1, 1, 1, 0, 0]);
    bytes
}

// Build a declared fixture variant, independently of the production encoder.
fn variant(
    subject: &[u8],
    concern: bool,
    target: Option<bool>,
    observation: u8,
    diplomacy: bool,
    concern_eligible: bool,
    disabled: bool,
) -> Vec<u8> {
    let original = record();
    let mut bytes = original[..398].to_vec();
    bytes.extend_from_slice(&(subject.len() as u32).to_le_bytes());
    bytes.extend_from_slice(subject);
    bytes.push(u8::from(concern));
    if concern {
        bytes.extend_from_slice(&[18; 16]);
    }
    bytes.extend_from_slice(&original[411..427]);
    bytes.extend_from_slice(&[
        observation,
        1,
        1,
        u8::from(diplomacy),
        u8::from(concern_eligible),
        u8::from(target.is_some()),
    ]);
    if let Some(hostile) = target {
        bytes.push(u8::from(hostile));
    }
    bytes.push(u8::from(disabled));
    bytes
}

#[test]
fn incomplete_or_extended_records_are_rejected_at_every_boundary() {
    let bytes = record();
    for end in 0..bytes.len() {
        assert_eq!(
            decode_action_input(&bytes[..end]),
            Err(Unavailable::Unsupported),
            "truncated prefix {end}"
        );
    }
    let mut extended = bytes;
    extended.push(0);
    assert_eq!(
        decode_action_input(&extended),
        Err(Unavailable::Unsupported)
    );
    assert_eq!(decode_action_input(&[0; 572]), Err(Unavailable::Limit));
}

#[test]
fn closed_tags_and_all_boolean_encodings_fail_closed() {
    let original = record();
    for offset in [0, 15, 16, 17] {
        let mut bytes = original.clone();
        bytes[offset] = 3;
        assert_eq!(
            decode_action_input(&bytes),
            Err(Unavailable::Unsupported),
            "header offset {offset}"
        );
    }
    for offset in [410, 427, 428, 429, 430, 431, 432, 433] {
        for value in 0..=u8::MAX {
            let admitted = match offset {
                427 => value == 1 || value == 2,
                428 | 429 => value == 1,
                _ => value <= 1,
            };
            if admitted {
                continue;
            }
            let mut bytes = original.clone();
            bytes[offset] = value;
            assert_eq!(
                decode_action_input(&bytes),
                Err(Unavailable::Unsupported),
                "offset {offset}, value {value}"
            );
        }
    }
    for offset in [433, 434] {
        for value in 2..=u8::MAX {
            let mut bytes = variant(b"hegemony", false, Some(true), 1, true, true, false);
            bytes[offset] = value;
            assert_eq!(decode_action_input(&bytes), Err(Unavailable::Unsupported));
        }
    }
}

#[test]
fn bounded_text_and_pinned_metadata_reject_invalid_records() {
    let original = record();
    for (offset, length) in [
        (18, 0_u32),
        (18, 41),
        (18, u32::MAX),
        (398, 0),
        (398, 129),
        (398, u32::MAX),
    ] {
        let mut bytes = original.clone();
        bytes[offset..offset + 4].copy_from_slice(&length.to_le_bytes());
        assert_eq!(decode_action_input(&bytes), Err(Unavailable::Limit));
    }
    for subject in [&b"\xff"[..], &b"hege\nony"[..], &b"e\xcc\x81"[..]] {
        let bytes = variant(subject, false, None, 1, true, true, false);
        assert_eq!(decode_action_input(&bytes), Err(Unavailable::Unsupported));
    }
    let mut wrong_source = original.clone();
    wrong_source[22] = b'b';
    assert_eq!(
        decode_action_input(&wrong_source),
        Err(Unavailable::MissingFact)
    );
    for index in 0..8 {
        let mut bytes = original.clone();
        bytes[62 + index * 32..94 + index * 32].fill(0);
        assert_eq!(decode_action_input(&bytes), Err(Unavailable::MissingFact));
    }
    let mut bytes = original;
    bytes[411..419].fill(0);
    assert_eq!(decode_action_input(&bytes), Err(Unavailable::MissingFact));
}

#[test]
fn option_and_fact_combinations_reproduce_only_synthetic_shadow_results() {
    let mut cases = 0;
    for concern in [false, true] {
        for target in [None, Some(false), Some(true)] {
            for observation in [1, 2] {
                for diplomacy in [false, true] {
                    for concern_eligible in [false, true] {
                        for disabled in [false, true] {
                            let bytes = variant(
                                b"hegemony",
                                concern,
                                target,
                                observation,
                                diplomacy,
                                concern_eligible,
                                disabled,
                            );
                            let decoded = decode_action_input(&bytes).unwrap();
                            assert_eq!(
                                decoded.facts,
                                MakePeaceEligibility {
                                    diplomacy_enabled: diplomacy,
                                    concern_can_make_peace: concern_eligible,
                                    target_hostile: target,
                                    faction_diplomacy_disabled: disabled,
                                }
                            );
                            assert_eq!(
                                decoded.metadata.concern_instance,
                                concern.then(|| EntityId::from_bytes([18; 16]))
                            );
                            assert_eq!(
                                decoded.metadata.provenance.observation,
                                if observation == 1 {
                                    Observation::Synthetic
                                } else {
                                    Observation::CapturedUnverified
                                }
                            );
                            assert_eq!(
                                encode_action_input(&decoded.metadata, &decoded.facts).unwrap(),
                                bytes
                            );
                            if observation == 2 {
                                assert_eq!(
                                    replay_synthetic_action(&bytes),
                                    Err(Unavailable::Unsupported)
                                );
                            } else {
                                let replay = replay_synthetic_action(&bytes).unwrap();
                                let expected = diplomacy
                                    && concern_eligible
                                    && target != Some(false)
                                    && !disabled;
                                assert_eq!(
                                    replay.output(),
                                    &ShadowOutput::MakePeaceEligibility(expected)
                                );
                                assert_eq!(replay.metadata(), &decoded.metadata);
                                assert_eq!(replay.binding_scope(), BindingScope::CopiedFacts);
                                assert_eq!(replay.authority(), ShadowAuthority::ShadowOnly);
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 96);
}

#[test]
fn largest_canonical_record_is_accepted_and_one_extra_byte_is_limited() {
    let mut bytes = variant(&[b'a'; 128], true, Some(true), 1, true, true, false);
    assert_eq!(bytes.len(), 571);
    let decoded = decode_action_input(&bytes).unwrap();
    assert_eq!(decoded.metadata.subject_faction, "a".repeat(128));
    assert_eq!(
        decoded.metadata.concern_instance,
        Some(EntityId::from_bytes([18; 16]))
    );
    assert_eq!(decoded.facts.target_hostile, Some(true));
    assert_eq!(
        replay_synthetic_action(&bytes).unwrap().output(),
        &ShadowOutput::MakePeaceEligibility(true)
    );
    bytes.push(0);
    assert_eq!(decode_action_input(&bytes), Err(Unavailable::Limit));
}
