use nf_contract::identity::{
    BranchId, CampaignId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};
use nf_nex_shadow::*;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    sync::atomic::Ordering,
    time::{Duration, Instant},
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
fn facts(fields: &[&str]) -> FirstProposalInput {
    assert_eq!(fields.len(), 24); // Raw supplied facts only, no expected-answer fields.
    let concern = super::support::war(&[
        fields[0], fields[2], fields[3], fields[4], fields[5], fields[6], fields[7], fields[8],
        "-", "-", "-", "-", "-",
    ]);
    let mut selected = super::support::peace(&[
        fields[0], fields[13], fields[14], fields[15], fields[16], "-", "-", "-",
    ]);
    selected.enemy = fields[17].into();
    selected.enemy_weariness = super::support::f(fields[18]);
    selected.own_events = super::support::f(fields[19]);
    selected.enemy_events = super::support::f(fields[20]);
    selected.enemy_is_player = fields[21].parse().unwrap();
    selected.own_weariness = super::support::f(fields[22]);
    selected.rules.minimum = super::support::f(fields[23]);
    FirstProposalInput {
        concern_operation: match fields[1] {
            "GENERATE" => WarOperation::Generate,
            "UPDATE" => WarOperation::Update,
            "VALID" => WarOperation::IsValid,
            _ => panic!("invalid operation"),
        },
        concern,
        eligibility: MakePeaceEligibility {
            diplomacy_enabled: fields[9].parse().unwrap(),
            concern_can_make_peace: fields[10].parse().unwrap(),
            target_hostile: if fields[11] == "-" {
                None
            } else {
                Some(fields[11].parse().unwrap())
            },
            faction_diplomacy_disabled: fields[12].parse().unwrap(),
        },
        selected,
    }
}
fn output(result: &FirstProposalOutput) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        result.concern.generated,
        result.concern.ended,
        result.concern.abort_current_action,
        result.concern.valid,
        super::support::modifiers(&result.concern.existing_priority),
        super::support::modifiers(&result.concern.writes),
        result.eligible,
        super::support::modifiers(std::slice::from_ref(&result.additional_priority)),
        match result.selected.decision {
            PeaceDecision::None => "NONE",
            PeaceDecision::Ceasefire => "CEASEFIRE",
            PeaceDecision::Treaty => "TREATY",
        },
        result.selected.consumed_draws,
        super::support::effects(&result.selected.effects)
    )
}

/// One independent combined JVM comparison and a real Rust-canonical-to-host public-fixture round trip.
#[test]
#[ignore = "requires reviewed NF_SHADOW_JAVA/NF_SHADOW_CLASSPATH_FILE/NF_SHADOW_NODE and fixed fixture custody"]
fn real_java_composite_facets_and_actual_rust_reproduction_match() {
    let metadata = metadata();
    let rows: Vec<_> =
        include_str!("../../../../java/nex-reference/src/test/resources/first-proposal-v1.tsv")
            .lines()
            .filter(|row| !row.starts_with('#') && !row.is_empty())
            .collect();
    assert_eq!(rows.len(), 12);
    let mut actual = String::new();
    let mut first = None;
    for row in rows {
        let fields: Vec<_> = row.split('|').collect();
        let input = facts(&fields);
        let before = input.clone();
        let mut session = ShadowSession::new(metadata.clone());
        let evaluation = session.evaluate_first_proposal(&input).unwrap();
        assert_eq!(evaluation.binding_scope(), BindingScope::CopiedFacts);
        assert_eq!(evaluation.authority(), ShadowAuthority::ShadowOnly);
        assert_eq!(
            session
                .accept_first_proposal(&evaluation, &metadata, &input)
                .unwrap(),
            evaluation.output()
        );
        assert_eq!(input, before);
        actual.push_str(&format!(
            "first_proposal|{}|{}\n",
            fields[0],
            output(evaluation.output())
        ));
        if first.is_none() {
            first = Some((input, evaluation));
        }
    }
    assert_eq!(
        super::oracle("first_proposal", "first-proposal-v1.tsv"),
        actual
    );
    let (input, evaluation) = first.unwrap();
    let original = input.clone();
    let bytes = synthetic_first_proposal_reproduction(&metadata, &input).unwrap();
    assert!(bytes.len() <= 65536);
    let digest = evaluation
        .input_digest()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.tmp/nex-first-proposal-reproductions-v1");
    let destination = root.join(format!("{digest}.bin"));
    let existing_stat = match fs::symlink_metadata(&destination) {
        Ok(stat) => {
            assert!(root.is_dir() && !root.symlink_metadata().unwrap().file_type().is_symlink());
            assert!(stat.is_file() && !stat.file_type().is_symlink());
            assert_eq!(stat.len(), bytes.len() as u64);
            assert_eq!(fs::read(&destination).unwrap(), bytes);
            Some(stat)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("canonical reproduction metadata failed: {error}"),
    };
    let expected_json = format!(
        "{{\"sha256\":\"{digest}\",\"byteLength\":{}}}\n",
        bytes.len()
    );
    assert_eq!(persist_public_fixture(&bytes), expected_json);
    assert!(root.is_dir() && !root.symlink_metadata().unwrap().file_type().is_symlink());
    let first_stat = fs::symlink_metadata(&destination).unwrap();
    if let Some(existing_stat) = &existing_stat {
        assert_eq!(
            first_stat.modified().unwrap(),
            existing_stat.modified().unwrap()
        );
    }
    assert!(first_stat.is_file() && !first_stat.file_type().is_symlink());
    assert_eq!(first_stat.len(), bytes.len() as u64);
    assert_eq!(fs::read(&destination).unwrap(), bytes);
    assert_eq!(persist_public_fixture(&bytes), expected_json);
    let second_stat = fs::symlink_metadata(&destination).unwrap();
    assert_eq!(second_stat.len(), first_stat.len());
    assert_eq!(
        second_stat.modified().unwrap(),
        first_stat.modified().unwrap()
    );
    assert_eq!(fs::read(&destination).unwrap(), bytes);
    assert_eq!(input, original);
    println!(
        "ACTUAL_PUBLIC_RUST_CANONICAL_REPRODUCTION sha256={digest} byteLength={} preexisting={}",
        bytes.len(),
        existing_stat.is_some()
    );
}
fn persist_public_fixture(bytes: &[u8]) -> String {
    assert!(bytes.len() <= 65536);
    let node =
        std::env::var_os("NF_SHADOW_NODE").expect("explicit managed Node executable required");
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    let root = base.join(format!(
        "oracle-{}-{}",
        std::process::id(),
        super::NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    assert!(root.starts_with(&base));
    fs::create_dir(&root).unwrap();
    let mut owned = super::Owned {
        child: None,
        root,
        base,
    };
    let input = owned.root.join("input");
    let mut input_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&input)
        .unwrap();
    input_file.write_all(bytes).unwrap(); // Only the newly owned exclusive scratch input.
    drop(input_file);
    let output = owned.root.join("out");
    let errors = owned.root.join("err");
    let out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    let err = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&errors)
        .unwrap();
    let mut command = Command::new(node);
    command
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/nex-first-proposal-repro-stdin.cjs"),
        )
        .stdin(fs::File::open(&input).unwrap())
        .stdout(out)
        .stderr(err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    owned.child = Some(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            fs::metadata(&output).unwrap().len() <= 65536
                && fs::metadata(&errors).unwrap().len() <= 4096,
            "bounded public-fixture host output"
        );
        if let Some(status) = owned.child.as_mut().unwrap().try_wait().unwrap() {
            assert!(
                status.success(),
                "host refused fixture: {}",
                fs::read_to_string(&errors).unwrap()
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "bounded public-fixture host deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(fs::metadata(&output).unwrap().len() <= 65536);
    assert_eq!(fs::metadata(&errors).unwrap().len(), 0);
    fs::read_to_string(output).unwrap()
}
