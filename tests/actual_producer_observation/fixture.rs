use nf_contract::identity::{BranchId, CampaignId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId};
use nf_nex_boundary::{Observation, Provenance, SOURCE_COMMIT};
use nf_nex_shadow::{MakePeaceEligibility, ShadowMetadata, ShadowOutput};

// Public RAM fixture only. Supplied bindings are not native/build/reference evidence.
pub fn fixture() -> (ShadowMetadata, MakePeaceEligibility, ShadowOutput) {
    let metadata = ShadowMetadata {
        provenance: Provenance {
            source_commit: SOURCE_COMMIT.to_owned(),
            source_digest: [1; 32],
            runtime_digest: [2; 32],
            ruleset_digest: [3; 32],
            merged_config_digest: [4; 32],
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
            runtime_session: RuntimeSession(1),
            frontier: EventSeq(0),
            observation: Observation::Synthetic,
        },
        subject_faction: "public-producer-fixture".to_owned(),
        concern_instance: None,
        provider: ProviderId::from_bytes([5; 16]),
        campaign: CampaignId::from_bytes([6; 16]),
        branch: BranchId::from_bytes([7; 16]),
        reference_digest: [8; 32],
        corpus_digest: [9; 32],
        implementation_digest: [10; 32],
        capture_policy_digest: [11; 32],
    };
    let input = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: Some(true),
        faction_diplomacy_disabled: false,
    };
    // Literal source derivation: no early false gate, then !false=true; reference=false.
    (metadata, input, ShadowOutput::MakePeaceEligibility(false))
}

pub fn public_original(metadata: &ShadowMetadata, input: &MakePeaceEligibility, reference: &ShadowOutput) -> Vec<u8> {
    let text = format!("PUBLIC-SYNTHETIC-RAM-FIXTURE-1\n{metadata:?}\n{input:?}\n{reference:?}\n");
    assert!(text.len() <= 16_384, "bounded public fixture capture");
    text.into_bytes()
}

pub fn assert_private_bindings_absent(text: &str, metadata: &ShadowMetadata) {
    for field in ["source_digest", "runtime_digest", "ruleset_digest", "merged_config_digest", "reference_digest", "corpus_digest", "implementation_digest", "capture_policy_digest", "runtime_session", "subject_faction"] {
        assert!(!text.contains(field), "private binding field in projection");
    }
    assert!(!text.contains(SOURCE_COMMIT));
    assert!(!text.contains(&metadata.subject_faction));
    for digest in [metadata.provenance.source_digest, metadata.provenance.runtime_digest, metadata.provenance.ruleset_digest, metadata.provenance.merged_config_digest, metadata.reference_digest, metadata.corpus_digest, metadata.implementation_digest, metadata.capture_policy_digest] {
        let hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        assert!(!text.contains(&hex), "private binding hex in projection");
        assert!(!text.contains(&format!("{digest:?}")), "private binding bytes in projection");
    }
}
