mod support;
use nf_contract::identity::{
    BranchId, CampaignId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};
use nf_nex_shadow::{
    Diagnostic, DiagnosticStream, Difference, MakePeaceEligibility, ShadowInput, ShadowMetadata,
    ShadowOutput, ShadowSession, WarOperation, compare_outputs,
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

#[test]
fn action_comparison_reports_decision_only_and_keeps_identity_separate() {
    let m = metadata();
    let facts = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: None,
        faction_diplomacy_disabled: false,
    };
    let mut session = ShadowSession::new(m.clone());
    let original = session.evaluate_action(&facts).unwrap();
    assert_eq!(original.output(), &ShadowOutput::MakePeaceEligibility(true));
    // Intended RED: current catch-all reports Kind even for the same actual typed action output.
    assert_eq!(
        compare_outputs(original.output(), original.output()),
        Vec::<Difference>::new()
    );

    // This is output comparison only. Equal output must not certify equal input or provenance.
    let changed_facts = MakePeaceEligibility {
        target_hostile: Some(true),
        ..facts
    };
    let changed = session.evaluate_action(&changed_facts).unwrap();
    assert_eq!(changed.output(), original.output());
    assert_ne!(changed.input_digest(), original.input_digest());
    assert_eq!(
        compare_outputs(original.output(), changed.output()),
        Vec::<Difference>::new()
    );
    let mut changed_metadata = m.clone();
    changed_metadata.provenance.frontier.0 += 1;
    let other = ShadowSession::new(changed_metadata)
        .evaluate_action(&facts)
        .unwrap();
    assert_eq!(other.output(), original.output());
    assert_ne!(other.input_digest(), original.input_digest());
    assert_eq!(
        compare_outputs(original.output(), other.output()),
        Vec::<Difference>::new()
    );

    let declined_facts = MakePeaceEligibility {
        concern_can_make_peace: false,
        ..facts
    };
    let declined = session.evaluate_action(&declined_facts).unwrap();
    assert_eq!(
        declined.output(),
        &ShadowOutput::MakePeaceEligibility(false)
    );
    assert_eq!(
        compare_outputs(original.output(), declined.output()),
        vec![Difference::Decision]
    );
    assert_eq!(
        compare_outputs(declined.output(), original.output()),
        vec![Difference::Decision]
    );
    let diagnostic = Diagnostic::from_evaluation(&original, Difference::Decision);
    assert_eq!(diagnostic.source_digest, m.provenance.source_digest);
    assert_eq!(diagnostic.config_digest, m.provenance.merged_config_digest);
    assert_eq!(diagnostic.input_digest, original.input_digest());
    assert_eq!(diagnostic.corpus_digest, m.corpus_digest);
    assert_eq!(diagnostic.implementation_digest, m.implementation_digest);
    let mut stream = DiagnosticStream::new(1, 161).unwrap();
    stream.push(diagnostic.clone());
    assert_eq!(stream.pop(), Some(diagnostic));
    assert_eq!(stream.pop(), None);

    let row = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .find(|row| row.starts_with("equal|"))
        .unwrap();
    let war = ShadowInput::War {
        operation: WarOperation::Generate,
        facts: support::war(&row.split('|').collect::<Vec<_>>()),
    };
    let foreign = session.evaluate(&war).unwrap();
    assert_eq!(
        compare_outputs(original.output(), foreign.output()),
        vec![Difference::Kind]
    );
    assert_eq!(
        compare_outputs(foreign.output(), original.output()),
        vec![Difference::Kind]
    );
}
