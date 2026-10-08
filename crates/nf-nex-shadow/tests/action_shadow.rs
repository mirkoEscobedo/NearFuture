use nf_contract::identity::{
    BranchId, CampaignId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};
use nf_nex_shadow::{
    BindingScope, MakePeaceEligibility, ShadowAuthority, ShadowMetadata, ShadowOutput,
    ShadowSession, action_input_digest, encode_action_input,
};

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
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn valid_no_target_action_diagnostic_is_typed_and_shadow_only() {
    let metadata = metadata();
    let facts = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: None,
        faction_diplomacy_disabled: false,
    };
    let original = facts;
    let row = include_str!("../fixtures/action-identity-v2.tsv")
        .lines()
        .nth(1)
        .expect("independent action identity vector");
    let fields: Vec<_> = row.split('|').collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0], "no-target");
    let encoded = encode_action_input(&metadata, &facts).unwrap();
    assert_eq!(encoded.len(), 434);
    assert_eq!(hex(&encoded), fields[1]);
    let digest = action_input_digest(&metadata, &facts).unwrap();
    assert_eq!(
        digest,
        [
            89, 16, 233, 68, 86, 10, 118, 85, 162, 50, 212, 140, 83, 234, 41, 52, 218, 116, 243,
            142, 229, 246, 185, 65, 165, 251, 208, 7, 208, 29, 247, 56
        ]
    );
    assert_eq!(hex(&digest), fields[2]);

    let mut session = ShadowSession::new(metadata.clone());
    let actual = session.evaluate_action(&facts);
    // Intended RED: supplied metadata is valid; the typed evaluator remains Unsupported.
    assert_eq!(
        actual.as_ref().map(|result| result.output()),
        Ok(&ShadowOutput::MakePeaceEligibility(true))
    );
    let result = actual.unwrap();
    assert_eq!(result.binding_scope(), BindingScope::CopiedFacts);
    assert_eq!(result.authority(), ShadowAuthority::ShadowOnly);
    assert_eq!(result.metadata(), &metadata);
    assert_eq!(result.input_digest(), digest);
    assert_eq!(facts, original);
}
