mod support;
use nf_contract::identity::*;
use nf_nex_boundary::{Observation, Provenance};
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
fn input() -> ShadowInput {
    let row = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .find(|v| v.starts_with("equal|"))
        .unwrap();
    ShadowInput::War {
        operation: WarOperation::Generate,
        facts: support::war(&row.split('|').collect::<Vec<_>>()),
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn private_identity_matches_independent_bytes_and_binds_all_context_changes() {
    let m = metadata();
    let input = input();
    let row = include_str!("../fixtures/identity-v1.tsv")
        .lines()
        .nth(1)
        .unwrap();
    let f: Vec<_> = row.split('|').collect();
    assert_eq!(hex(&encode_input(&m, &input).unwrap()), f[1]);
    assert_eq!(hex(&input_digest(&m, &input).unwrap()), f[2]);
    let original = input_digest(&m, &input).unwrap();
    for mutation in 0..16 {
        let mut changed = m.clone();
        match mutation {
            0 => changed.provenance.runtime_session.0 += 1,
            1 => changed.provenance.frontier.0 += 1,
            2 => changed.provenance.observation = Observation::CapturedUnverified,
            3 => changed.provenance.source_digest[0] ^= 1,
            4 => changed.provenance.runtime_digest[0] ^= 1,
            5 => changed.provenance.ruleset_digest[0] ^= 1,
            6 => changed.provenance.merged_config_digest[0] ^= 1,
            7 => changed.reference_digest[0] ^= 1,
            8 => changed.corpus_digest[0] ^= 1,
            9 => changed.implementation_digest[0] ^= 1,
            10 => changed.capture_policy_digest[0] ^= 1,
            11 => changed.branch = BranchId::from_bytes([18; 16]),
            12 => changed.campaign = CampaignId::from_bytes([18; 16]),
            13 => changed.provider = ProviderId::from_bytes([18; 16]),
            14 => changed.subject_faction = "tritachyon".into(),
            15 => changed.concern_instance = Some(EntityId::from_bytes([19; 16])),
            _ => unreachable!(),
        };
        assert_ne!(input_digest(&changed, &input).unwrap(), original);
    }
    let mut absent = m;
    absent.provenance.runtime_digest = [0; 32];
    assert_eq!(input_digest(&absent, &input), Err(Unavailable::MissingFact));
}
#[test]
fn shadow_publication_rejects_stale_input_and_invalidated_lifecycle() {
    let m = metadata();
    let input = input();
    let output = evaluate_shadow(&m, &input).unwrap();
    assert_eq!(output.authority(), ShadowAuthority::ShadowOnly);
    let mut session = ShadowSession::new(m.clone());
    assert!(session.accept(&output, &m, &input).is_ok());
    let mut drift = input.clone();
    if let ShadowInput::War { facts, .. } = &mut drift {
        facts.minimum = support::float(6000.0);
    }
    assert_eq!(session.accept(&output, &m, &drift), Err(Unavailable::Stale));
    let mut next = m.clone();
    next.provenance.runtime_session.0 += 1;
    assert_eq!(
        ShadowSession::new(next.clone()).accept(&output, &next, &input),
        Err(Unavailable::Stale)
    );
    session.invalidate();
    assert_eq!(
        session.accept(&output, &m, &input),
        Err(Unavailable::Disabled)
    );
}

#[test]
fn provider_failure_disables_optional_work_and_reproduction_rejects_captured_inputs() {
    let m = metadata();
    let mut session = ShadowSession::new(m.clone());
    let mut invalid = input();
    if let ShadowInput::War { facts, .. } = &mut invalid {
        facts.priority.tags[0] = "x".repeat(257);
    }
    assert_eq!(session.evaluate(&invalid), Err(Unavailable::Limit));
    assert_eq!(session.evaluate(&input()), Err(Unavailable::Disabled));
    let bytes = synthetic_reproduction(&m, &input()).unwrap();
    assert_eq!(bytes, encode_input(&m, &input()).unwrap());
    let mut captured = m;
    captured.provenance.observation = Observation::CapturedUnverified;
    assert_eq!(
        synthetic_reproduction(&captured, &input()),
        Err(Unavailable::Unsupported)
    );
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join(format!("public-repro-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut file, &bytes).unwrap();
    drop(file);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn current_owner_binding_changes_reject_old_result_with_identical_numeric_facts() {
    let original = metadata();
    let input = input();
    let result = evaluate_shadow(&original, &input).unwrap();
    let session = ShadowSession::new(original.clone());
    for mutation in 0..8 {
        let mut current = original.clone();
        match mutation {
            0 => current.provenance.runtime_session.0 += 1,
            1 => current.provenance.frontier.0 += 1,
            2 => current.provenance.merged_config_digest[0] ^= 1,
            3 => current.subject_faction = "tritachyon".into(),
            4 => current.concern_instance = Some(EntityId::from_bytes([19; 16])),
            5 => current.provenance.source_digest[0] ^= 1,
            6 => current.capture_policy_digest[0] ^= 1,
            7 => current.provenance.ruleset_digest[0] ^= 1,
            _ => unreachable!(),
        }
        assert_eq!(
            session.accept(&result, &current, &input),
            Err(Unavailable::Stale)
        );
    }
    let mut invalid = original;
    invalid.provenance.runtime_session.0 = 0;
    assert_eq!(
        session.accept(&result, &invalid, &input),
        Err(Unavailable::MissingFact)
    );
}
