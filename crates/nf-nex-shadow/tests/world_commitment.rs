mod world_support;
use nf_nex_boundary::*;
use nf_nex_shadow::*;
fn hex(b: &[u8]) -> String {
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn golden(name: &str) -> Vec<&'static str> {
    include_str!("../fixtures/world-v1.tsv")
        .lines()
        .find(|l| l.starts_with(&format!("{name}|")))
        .unwrap()
        .split('|')
        .collect()
}
#[test]
fn full_world_and_binding_match_independent_standard_crypto_goldens() {
    for name in [
        "baseline",
        "rich",
        "rich_traits_reordered",
        "rich_concerns_reordered",
        "rich_disposition_absent",
        "baseline_positive_zero",
    ] {
        let mut s = if name.starts_with("rich") {
            world_support::rich_snapshot()
        } else {
            world_support::snapshot()
        };
        match name {
            "rich_traits_reordered" => s.factions[0].traits.reverse(),
            "rich_concerns_reordered" => s.concerns.reverse(),
            "rich_disposition_absent" => s.relations[0].disposition = None,
            "baseline_positive_zero" => s.concerns[0].cooldown = FloatBits::new(0).unwrap(),
            _ => {}
        }
        let world = NexWorld::admit(s).unwrap();
        assert_eq!(
            hex(&commit_world(&world).unwrap().digest()),
            golden(name)[2],
            "{name}"
        );
    }
    let s = world_support::snapshot();
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let w = NexWorld::admit(s).unwrap();
    let mut session = WorldShadowSession::new(m, &w).unwrap();
    let result = session
        .evaluate_war(&w, WarOperation::Update, target)
        .unwrap();
    assert_eq!(
        hex(&result.binding_digest()),
        golden("baseline_update_binding")[2]
    );
    assert_eq!(
        hex(&result.copied_input_digest()),
        golden("baseline_update_copied")[2]
    );
    assert_eq!(result.authority(), ShadowAuthority::ShadowOnly);
}
#[test]
fn exact_cumulative_limit_is_admitted_and_one_more_never_mints_world_token() {
    let mut s = world_support::rich_snapshot();
    for i in 2..64 {
        s.factions.push(FactionView {
            id: format!("faction{i}"),
            live: true,
            traits: vec![],
            diplomatic_alignment: FloatBits::new(0).unwrap(),
            priority_multipliers: vec![],
        });
    }
    s.relations.clear();
    for a in &s.factions {
        for b in &s.factions {
            if a.id != b.id {
                s.relations.push(RelationView {
                    from: a.id.clone(),
                    to: b.id.clone(),
                    relationship: FloatBits::new(0).unwrap(),
                    disposition: None,
                    hostile: false,
                });
            }
        }
    }
    s.timers.draws = (0..4070)
        .map(|i| RandomDraw {
            purpose: "peace-choice".into(),
            ordinal: i,
            value: DoubleBits::new(0).unwrap(),
        })
        .collect();
    let world = NexWorld::admit(s.clone()).unwrap();
    assert!(commit_world(&world).is_ok());
    s.timers.draws.push(RandomDraw {
        purpose: "peace-choice".into(),
        ordinal: 4070,
        value: DoubleBits::new(0).unwrap(),
    });
    assert_eq!(NexWorld::admit(s), Err(BoundaryError::Limit));
}
#[test]
fn unknown_missing_or_noncanonical_native_facts_cannot_mint_commitments() {
    for mutation in 0..10 {
        let mut s = world_support::rich_snapshot();
        match mutation {
            0 => s.provenance.source_commit = "unknown".into(),
            1 => s.concerns[0].definition.module = Module::Economic,
            2 => s.concerns[0].definition.class_path = "extension.Unknown".into(),
            3 => s.extensions.complete = false,
            4 => s.extensions.listeners.push(Registration {
                class_path: "extension.Listener".into(),
                origin: "unknown".into(),
            }),
            5 => s.extensions.direct_calls.push(Registration {
                class_path: "extension.Call".into(),
                origin: "unknown".into(),
            }),
            6 => s
                .extensions
                .other_definitions
                .push(s.concern_config.definition.clone()),
            7 => s.factions[0].traits.push("stalwart".into()),
            8 => s.factions[0].id = "a\u{301}".into(),
            9 => s.concerns[0].market_id = Some("missing".into()),
            _ => unreachable!(),
        }
        assert!(
            NexWorld::admit(s).is_err(),
            "invalid native fact {mutation}"
        );
    }
}
