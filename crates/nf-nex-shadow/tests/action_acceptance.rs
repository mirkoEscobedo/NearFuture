mod support;
use nf_contract::identity::{
    BranchId, CampaignId, EntityId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};
use nf_nex_shadow::{
    BindingScope, MakePeaceEligibility, ShadowAuthority, ShadowInput, ShadowMetadata, ShadowOutput,
    ShadowSession, Unavailable, WarOperation, action_input_digest,
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
fn current_action_binding_rejects_stale_equal_boolean_and_lifecycle_changes() {
    let m = metadata();
    let facts = MakePeaceEligibility {
        diplomacy_enabled: true,
        concern_can_make_peace: true,
        target_hostile: None,
        faction_diplomacy_disabled: false,
    };
    let mut session = ShadowSession::new(m.clone());
    let result = session.evaluate_action(&facts).unwrap();
    assert_eq!(result.output(), &ShadowOutput::MakePeaceEligibility(true));
    assert_eq!(result.binding_scope(), BindingScope::CopiedFacts);
    assert_eq!(result.authority(), ShadowAuthority::ShadowOnly);
    assert_eq!(
        result.input_digest(),
        action_input_digest(&m, &facts).unwrap()
    );
    // Intended RED: a real V2 evaluation is complete; the new acceptance stub is Unsupported.
    assert_eq!(
        session.accept_action(&result, &m, &facts),
        Ok(&ShadowOutput::MakePeaceEligibility(true))
    );

    // These are supplied current-context changes, not authenticated live identity claims.
    for mutation in 0..16 {
        let mut current = m.clone();
        match mutation {
            0 => current.provenance.runtime_session.0 += 1,
            1 => current.provenance.frontier.0 += 1,
            2 => current.provenance.observation = Observation::CapturedUnverified,
            3 => current.provenance.source_digest[0] ^= 1,
            4 => current.provenance.runtime_digest[0] ^= 1,
            5 => current.provenance.ruleset_digest[0] ^= 1,
            6 => current.provenance.merged_config_digest[0] ^= 1,
            7 => current.reference_digest[0] ^= 1,
            8 => current.corpus_digest[0] ^= 1,
            9 => current.implementation_digest[0] ^= 1,
            10 => current.capture_policy_digest[0] ^= 1,
            11 => current.branch = BranchId::from_bytes([18; 16]),
            12 => current.campaign = CampaignId::from_bytes([18; 16]),
            13 => current.provider = ProviderId::from_bytes([18; 16]),
            14 => current.subject_faction = "tritachyon".into(),
            15 => current.concern_instance = Some(EntityId::from_bytes([19; 16])),
            _ => unreachable!(),
        }
        assert_eq!(
            session.accept_action(&result, &current, &facts),
            Err(Unavailable::Stale)
        );
    }
    let changed = MakePeaceEligibility {
        target_hostile: Some(true),
        ..facts
    };
    let changed_result = session.evaluate_action(&changed).unwrap();
    assert_eq!(changed_result.output(), result.output());
    assert_ne!(changed_result.input_digest(), result.input_digest());
    assert_eq!(
        session.accept_action(&result, &m, &changed),
        Err(Unavailable::Stale)
    );

    let mut invalid = m.clone();
    invalid.provenance.runtime_session.0 = 0;
    assert_eq!(
        session.accept_action(&result, &invalid, &facts),
        Err(Unavailable::MissingFact)
    );
    let other_owner = ShadowSession::new({
        let mut next = m.clone();
        next.provenance.runtime_session.0 += 1;
        next
    });
    assert_eq!(
        other_owner.accept_action(&result, &m, &facts),
        Err(Unavailable::Stale)
    );

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
        session.accept_action(&foreign, &m, &facts),
        Err(Unavailable::Unsupported)
    );
    assert_eq!(session.accept(&result, &m, &war), Err(Unavailable::Stale));
    assert_eq!(
        session.accept_action(&result, &m, &facts),
        Ok(&ShadowOutput::MakePeaceEligibility(true))
    );

    session.invalidate();
    assert_eq!(
        session.accept_action(&result, &m, &facts),
        Err(Unavailable::Disabled)
    );
    assert_eq!(session.evaluate_action(&facts), Err(Unavailable::Disabled));
    let mut provider = ShadowSession::new(m.clone());
    let provider_result = provider.evaluate_action(&facts).unwrap();
    provider.disable_provider();
    assert_eq!(
        provider.accept_action(&provider_result, &m, &facts),
        Err(Unavailable::Disabled)
    );
    assert_eq!(provider.evaluate_action(&facts), Err(Unavailable::Disabled));
}
