mod support;
use nf_nex_boundary::*;

#[test]
fn closed_mapping_keeps_known_peace_route_unavailable() {
    assert_eq!(
        classify_definition(&support::concern_definition()),
        Ok(Surface::WarWearinessInput)
    );
    let action = Definition {
        id: "makePeace".into(),
        class_path: "exerelin.campaign.ai.action.MakePeaceAction".into(),
        module: Module::Diplomatic,
    };
    assert_eq!(
        classify_definition(&action),
        Ok(Surface::MakePeaceUnavailable)
    );
    let missing = Definition {
        id: "".into(),
        ..action
    };
    assert!(matches!(
        classify_definition(&missing),
        Err(BoundaryError::Limit)
    ));
}

#[test]
fn provenance_and_unknown_semantic_facts_are_explicit_errors() {
    let mut snapshot = support::snapshot();
    snapshot.provenance.source_commit = "different-source".into();
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::InvalidFact("source pin"))
    );
    let mut snapshot = support::snapshot();
    snapshot.provenance.runtime_digest = [0; 32];
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::MissingFact("provenance identity"))
    );
    let mut snapshot = support::snapshot();
    snapshot.weariness.enemies.push("missing-faction".into());
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::MissingFact("faction reference"))
    );
    let mut snapshot = support::snapshot();
    snapshot.factions[0].traits.push("unknown-trait".into());
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::InvalidFact("faction trait"))
    );
    let mut snapshot = support::snapshot();
    snapshot.concerns[0].priority[0].id = "listener-written-priority".into();
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::InvalidFact("priority modifier"))
    );
}

#[test]
fn nested_views_share_a_finite_entry_budget() {
    let mut snapshot = support::snapshot();
    let template = snapshot.factions[0].clone();
    for index in 1..64 {
        let mut faction = template.clone();
        faction.id = format!("faction-{index}");
        snapshot.factions.push(faction);
    }
    for from in &snapshot.factions {
        for to in &snapshot.factions {
            if from.id != to.id {
                snapshot.relations.push(RelationView {
                    from: from.id.clone(),
                    to: to.id.clone(),
                    relationship: support::number(0),
                    disposition: None,
                    hostile: false,
                });
            }
        }
    }
    snapshot.timers.draws = (0..4096)
        .map(|ordinal| RandomDraw {
            purpose: "peace-choice".into(),
            ordinal,
            value: DoubleBits::new(0).unwrap(),
        })
        .collect();
    assert_eq!(NexWorld::admit(snapshot), Err(BoundaryError::Limit));
    let mut inventory = support::snapshot().extensions;
    inventory.listeners = vec![
        Registration {
            class_path: "third.party.Listener".into(),
            origin: "mod-a".into()
        };
        257
    ];
    assert_eq!(assess_extensions(&inventory), Err(BoundaryError::Limit));
}

#[test]
fn finite_raw_float_mutations_preserve_bits_and_never_promote_capture() {
    let mut state = 0x2ca1_d67fu32;
    for _ in 0..10_000 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        if let Ok(value) = FloatBits::new(state) {
            let mut snapshot = support::snapshot();
            snapshot.weariness.adjusted = value;
            snapshot.provenance.observation = Observation::CapturedUnverified;
            let world = NexWorld::admit(snapshot).unwrap();
            assert_eq!(world.snapshot().weariness.adjusted.bits(), state);
            assert_eq!(world.assessment().authority, Authority::ShadowOnly);
            assert_eq!(world.assessment().runtime, RuntimeCertification::Unobserved);
        }
    }
}
