mod support;
use nf_contract::identity::*;
use nf_nex_boundary::{DoubleBits, FloatBits, Modifier, ModifierKind, Observation, Provenance};
use nf_nex_shadow::*;

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
fn input() -> FirstProposalInput {
    // Public supplied facts only; the independent literal below does not read golden columns.
    FirstProposalInput {
        concern_operation: WarOperation::Generate,
        concern: support::war(
            &"active_other_class|459c4000|459c4000|false|false|other|0|-|true|false|false|true|-"
                .split('|')
                .collect::<Vec<_>>(),
        ),
        eligibility: MakePeaceEligibility {
            diplomacy_enabled: true,
            concern_can_make_peace: true,
            target_hostile: None,
            faction_diplomacy_disabled: false,
        },
        selected: support::peace(
            &"chance_above_double_precision|none|false|3fe0000000000001|-|NONE|1|-"
                .split('|')
                .collect::<Vec<_>>(),
        ),
    }
}
fn modifier(id: &str, kind: ModifierKind, bits: u32) -> Modifier {
    Modifier {
        id: id.into(),
        kind,
        value: FloatBits::new(bits).unwrap(),
    }
}
fn literal() -> FirstProposalOutput {
    FirstProposalOutput {
        concern: WarResult {
            generated: true,
            ended: false,
            abort_current_action: false,
            valid: true,
            existing_priority: vec![modifier("value", ModifierKind::Flat, 0x3f800000)],
            writes: vec![
                modifier("value", ModifierKind::Flat, 0x42480000),
                modifier("alignment_diplomatic", ModifierKind::Multiplier, 0x3f900000),
            ],
        },
        eligible: true,
        additional_priority: modifier("weariness", ModifierKind::Multiplier, 0x3f800000),
        selected: PeaceResult {
            decision: PeaceDecision::None,
            consumed_draws: 1,
            effects: vec![],
        },
    }
}
#[test]
fn equal_output_changed_selected_input_must_be_stale() {
    let metadata = metadata();
    let original = input();
    let mut session = ShadowSession::new(metadata.clone());
    let result = session.evaluate_first_proposal(&original).unwrap();
    assert_eq!(result.output(), &literal());
    assert_eq!(result.binding_scope(), BindingScope::CopiedFacts);
    assert_eq!(result.authority(), ShadowAuthority::ShadowOnly);
    let mut changed = original.clone();
    changed.selected.enemy = "pirates".into();
    let fresh = ShadowSession::new(metadata.clone())
        .evaluate_first_proposal(&changed)
        .unwrap();
    assert_eq!(fresh.output(), result.output()); // Equal output must never certify identical raw input.
    assert_eq!(
        session.accept_first_proposal(&result, &metadata, &changed),
        Err(Unavailable::Stale)
    );
    assert_eq!(original, input());
}
#[test]
fn full_framed_bytes_bind_raw_facets_even_when_unused_or_equal() {
    let metadata = metadata();
    let original = input();
    let bytes = encode_first_proposal_input(&metadata, &original).unwrap();
    assert!(bytes.starts_with(b"NF-NEX-FIRST-PROPOSAL-1\0"));
    assert!(bytes.len() <= 65536);
    for mutation in 0..6 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.concern_operation = WarOperation::Update,
            1 => changed.eligibility.target_hostile = Some(true),
            2 => changed.selected.enemy = "pirates".into(),
            3 => changed.selected.own_events = FloatBits::new(0x80000000).unwrap(),
            4 => changed.selected.draws[0].value = DoubleBits::new(0x3fe0000000000002).unwrap(),
            5 => changed.concern.priority.existing[0].value = FloatBits::new(0x3f000000).unwrap(),
            _ => unreachable!(),
        }
        assert_ne!(
            encode_first_proposal_input(&metadata, &changed).unwrap(),
            bytes
        );
    }
    for mutation in 0..4 {
        let mut changed = metadata.clone();
        match mutation {
            0 => changed.provenance.source_digest[0] ^= 1,
            1 => changed.provenance.merged_config_digest[0] ^= 1,
            2 => changed.corpus_digest[0] ^= 1,
            3 => changed.implementation_digest[0] ^= 1,
            _ => unreachable!(),
        }
        assert_ne!(
            encode_first_proposal_input(&changed, &original).unwrap(),
            bytes
        );
    }
}
#[test]
fn existing_active_fence_rejects_lifecycle_changes_and_disables_on_error() {
    let metadata = metadata();
    let original = input();
    let mut session = ShadowSession::new(metadata.clone());
    let result = session.evaluate_first_proposal(&original).unwrap();
    for mutation in 0..3 {
        let mut current = metadata.clone();
        match mutation {
            0 => current.provenance.runtime_session.0 += 1,
            1 => current.provenance.frontier.0 += 1,
            2 => current.branch = BranchId::from_bytes([99; 16]),
            _ => unreachable!(),
        }
        assert_eq!(
            session.accept_first_proposal(&result, &current, &original),
            Err(Unavailable::Stale)
        );
    }
    session.invalidate();
    assert_eq!(
        session.accept_first_proposal(&result, &metadata, &original),
        Err(Unavailable::Disabled)
    );
    let mut session = ShadowSession::new(metadata);
    let mut unsupported = original.clone();
    unsupported.selected.enemy_is_player = true;
    assert_eq!(
        session.evaluate_first_proposal(&unsupported),
        Err(Unavailable::Unsupported)
    );
    assert_eq!(
        session.evaluate_first_proposal(&original),
        Err(Unavailable::Disabled)
    );
    assert_eq!(original, input());
}
#[test]
fn facet_kind_differences_and_bounded_diagnostics_keep_exact_identities() {
    let metadata = metadata();
    let result = ShadowSession::new(metadata.clone())
        .evaluate_first_proposal(&input())
        .unwrap();
    let observation = FirstProposalObservation {
        concern: ShadowOutput::MakePeaceEligibility(true),
        eligibility: ShadowOutput::SelectedPeace(literal().selected.clone()),
        additional_priority: modifier("weariness", ModifierKind::Multiplier, 0x40a00000),
        selected: ShadowOutput::War(literal().concern),
    };
    let differences = compare_first_proposal_outputs(&observation, result.output());
    assert_eq!(
        differences,
        vec![
            Difference::FirstConcernKind,
            Difference::FirstEligibilityKind,
            Difference::FirstActionPriority,
            Difference::FirstProposalKind
        ]
    );
    let mut stream = DiagnosticStream::new(2, 322).unwrap();
    for difference in differences {
        stream.push(Diagnostic::from_first_proposal(&result, difference));
    }
    assert_eq!(stream.dropped(), 2);
    let first = stream.pop().unwrap();
    let second = stream.pop().unwrap();
    assert_ne!(first.difference, second.difference);
    assert_eq!(first.input_digest, result.input_digest());
    assert_eq!(first.source_digest, metadata.provenance.source_digest);
    assert_eq!(
        first.config_digest,
        metadata.provenance.merged_config_digest
    );
    assert_eq!(first.corpus_digest, metadata.corpus_digest);
    assert_eq!(first.implementation_digest, metadata.implementation_digest);
    assert!(stream.pop().is_none());
}
#[test]
fn synthetic_reproduction_is_value_only_and_refuses_captured_inputs() {
    let metadata = metadata();
    let original = input();
    let bytes = synthetic_first_proposal_reproduction(&metadata, &original).unwrap();
    assert_eq!(
        bytes,
        encode_first_proposal_input(&metadata, &original).unwrap()
    );
    let mut captured = metadata;
    captured.provenance.observation = Observation::CapturedUnverified;
    assert_eq!(
        synthetic_first_proposal_reproduction(&captured, &original),
        Err(Unavailable::Unsupported)
    );
    assert_eq!(original, input());
}
#[test]
fn combined_outputs_and_identities_repeat_across_worker_completion_orders() {
    let metadata = metadata();
    let jobs: Vec<_> = (0..4)
        .map(|index| {
            let mut supplied = input();
            supplied.selected.enemy = format!("enemy_{index}");
            if index % 2 == 0 {
                supplied.selected.draws[0].value = DoubleBits::new(0x3fe0000000000000).unwrap();
            }
            if index == 1 {
                supplied.eligibility.faction_diplomacy_disabled = true;
            }
            if index == 3 {
                supplied.concern_operation = WarOperation::Update;
            }
            supplied
        })
        .collect();
    let before = jobs.clone();
    let run = |input: &FirstProposalInput| {
        ShadowSession::new(metadata.clone())
            .evaluate_first_proposal(input)
            .unwrap()
    };
    let expected: Vec<_> = jobs.iter().map(run).collect();
    for workers in [1, 2, 4] {
        for _ in 0..3 {
            let mut actual = std::thread::scope(|scope| {
                let handles: Vec<_> = (0..workers)
                    .map(|worker| {
                        let jobs = &jobs;
                        let run = &run;
                        scope.spawn(move || {
                            jobs.iter()
                                .enumerate()
                                .rev()
                                .filter(|(i, _)| i % workers == worker)
                                .map(|(i, input)| (i, run(input)))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .rev()
                    .flat_map(|handle| handle.join().unwrap())
                    .collect::<Vec<_>>()
            });
            actual.sort_by_key(|(i, _)| *i);
            assert_eq!(
                actual
                    .into_iter()
                    .map(|(_, result)| result)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
    assert_eq!(jobs, before);
}

#[test]
fn composite_provider_error_disables_all_diagnostic_entrypoints() {
    let metadata = metadata();
    let supplied = input();
    let before = supplied.clone();
    let war = ShadowInput::War {
        operation: supplied.concern_operation,
        facts: supplied.concern.clone(),
    };
    let mut session = ShadowSession::new(metadata.clone());
    let composite_result = session.evaluate_first_proposal(&supplied).unwrap();
    let war_result = session.evaluate(&war).unwrap();
    let action_result = session.evaluate_action(&supplied.eligibility).unwrap();
    assert_eq!(
        session.accept(&war_result, &metadata, &war),
        Ok(&ShadowOutput::War(literal().concern))
    );
    assert_eq!(
        session.accept_action(&action_result, &metadata, &supplied.eligibility),
        Ok(&ShadowOutput::MakePeaceEligibility(true))
    );
    assert_eq!(
        session.accept_first_proposal(&composite_result, &metadata, &supplied),
        Ok(&literal())
    );

    let mut unsupported = supplied.clone();
    unsupported.selected.enemy_is_player = true;
    let unsupported_before = unsupported.clone();
    assert_eq!(
        session.evaluate_first_proposal(&unsupported),
        Err(Unavailable::Unsupported)
    );
    assert_eq!(session.evaluate(&war), Err(Unavailable::Disabled));
    assert_eq!(
        session.evaluate_action(&supplied.eligibility),
        Err(Unavailable::Disabled)
    );
    assert_eq!(
        session.evaluate_first_proposal(&supplied),
        Err(Unavailable::Disabled)
    );
    assert_eq!(
        session.accept(&war_result, &metadata, &war),
        Err(Unavailable::Disabled)
    );
    assert_eq!(
        session.accept_action(&action_result, &metadata, &supplied.eligibility),
        Err(Unavailable::Disabled)
    );
    assert_eq!(
        session.accept_first_proposal(&composite_result, &metadata, &supplied),
        Err(Unavailable::Disabled)
    );
    assert_eq!(supplied, before);
    assert_eq!(unsupported, unsupported_before);
}
