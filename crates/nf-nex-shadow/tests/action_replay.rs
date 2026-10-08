use nf_contract::identity::{
    BranchId, CampaignId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};
use nf_nex_shadow::{
    BindingScope, DecodedActionInput, MakePeaceEligibility, ShadowAuthority, ShadowMetadata,
    ShadowOutput, Unavailable, action_input_digest, decode_action_input, encode_action_input,
    replay_synthetic_action,
};
use sha2::{Digest, Sha256};

fn metadata() -> ShadowMetadata {
    ShadowMetadata {
        subject_faction: "hegemony".into(),
        concern_instance: None,
        provenance: Provenance {
            source_commit: nf_nex_boundary::SOURCE_COMMIT.into(),
            source_digest: [1; 32],
            runtime_digest: [2; 32],
            ruleset_digest: [3; 32],
            merged_config_digest: [4; 32],
            universe: UniverseId::from_bytes([11; 16]),
            history: HistoryId::from_bytes([12; 16]),
            runtime_session: RuntimeSession(13),
            frontier: EventSeq(14),
            observation: Observation::Synthetic,
        },
        provider: ProviderId::from_bytes([15; 16]),
        campaign: CampaignId::from_bytes([16; 16]),
        branch: BranchId::from_bytes([17; 16]),
        reference_digest: [5; 32],
        corpus_digest: [6; 32],
        implementation_digest: [7; 32],
        capture_policy_digest: [8; 32],
    }
}

#[test]
fn independent_action_record_decodes_and_replays_synthetic_only() {
    let m = metadata();
    let facts = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: None,
        faction_diplomacy_disabled: false,
    };
    let fields: Vec<_> = include_str!("../fixtures/action-identity-v2.tsv")
        .lines()
        .nth(1)
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0], "no-target");
    assert_eq!(fields[1].len(), 868);
    let bytes: Vec<u8> = fields[1]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes.len(), 434);
    assert_eq!(&bytes[..16], b"NF-NEX-SHADOW-2\0");
    assert_eq!(&bytes[16..18], &[2, 0]);
    let independent_digest: [u8; 32] = Sha256::digest(&bytes).into();
    assert_eq!(
        independent_digest,
        [
            89, 16, 233, 68, 86, 10, 118, 85, 162, 50, 212, 140, 83, 234, 41, 52, 218, 116, 243,
            142, 229, 246, 185, 65, 165, 251, 208, 7, 208, 29, 247, 56
        ]
    );
    assert_eq!(
        fields[2],
        "5910e944560a7655a232d48c53ea2934da74f38ee5f6b941a5fbd007d01df738"
    );
    let original_bytes = bytes.clone();
    // Intended RED: the valid independent record is recognized; body decode is still Unsupported.
    assert_eq!(
        decode_action_input(&bytes),
        Ok(DecodedActionInput {
            metadata: m.clone(),
            facts
        })
    );
    let decoded = decode_action_input(&bytes).unwrap();
    assert_eq!(
        encode_action_input(&decoded.metadata, &decoded.facts).unwrap(),
        original_bytes
    );
    assert_eq!(
        action_input_digest(&decoded.metadata, &decoded.facts).unwrap(),
        independent_digest
    );
    let replay = replay_synthetic_action(&bytes).unwrap();
    assert_eq!(replay.output(), &ShadowOutput::MakePeaceEligibility(true));
    assert_eq!(replay.metadata(), &m);
    assert_eq!(replay.input_digest(), independent_digest);
    assert_eq!(replay.binding_scope(), BindingScope::CopiedFacts);
    assert_eq!(replay.authority(), ShadowAuthority::ShadowOnly);
    assert_eq!(bytes, original_bytes);

    let mut captured = m;
    captured.provenance.observation = Observation::CapturedUnverified;
    let captured_bytes = encode_action_input(&captured, &facts).unwrap();
    assert_eq!(
        decode_action_input(&captured_bytes),
        Ok(DecodedActionInput {
            metadata: captured,
            facts
        })
    );
    assert_eq!(
        replay_synthetic_action(&captured_bytes),
        Err(Unavailable::Unsupported)
    );
}
