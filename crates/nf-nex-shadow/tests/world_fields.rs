mod world_support;
use nf_contract::identity::*;
use nf_nex_boundary::*;
use nf_nex_shadow::*;
fn f() -> FloatBits {
    FloatBits::new(1).unwrap()
}
#[test]
fn every_admitted_mutable_scalar_and_presence_changes_full_world_identity() {
    let original = world_support::rich_snapshot();
    let digest = commit_world(&NexWorld::admit(original.clone()).unwrap()).unwrap();
    for mutation in 0..66 {
        let mut s = original.clone();
        match mutation {
            0 => s.provenance.source_digest[0] ^= 1,
            1 => s.provenance.runtime_digest[0] ^= 1,
            2 => s.provenance.ruleset_digest[0] ^= 1,
            3 => s.provenance.merged_config_digest[0] ^= 1,
            4 => s.provenance.universe = UniverseId::from_bytes([30; 16]),
            5 => s.provenance.history = HistoryId::from_bytes([31; 16]),
            6 => s.provenance.runtime_session.0 += 1,
            7 => s.provenance.frontier.0 += 1,
            8 => s.provenance.observation = Observation::CapturedUnverified,
            9 => {
                s.subject_faction = "tritachyon".into();
                s.weariness.enemies.clear();
            }
            10 => s.factions[0].live = false,
            11 => s.factions[0].traits.pop().map(|_| ()).unwrap(),
            12 => s.factions[0].diplomatic_alignment = f(),
            13 => s.factions[0].priority_multipliers[0].1 = f(),
            14 => s.relations[0].relationship = f(),
            15 => s.relations[0].disposition = None,
            16 => s.relations[0].hostile = false,
            17 => s.strengths[0].markets[0].size += 1,
            18 => s.strengths[0].markets[0].strength = f(),
            19 => s.strengths[0].fleet_strength = f(),
            20 => s.weariness.manager_present = false,
            21 => s.weariness.map_entry_present = false,
            22 => s.weariness.raw = f(),
            23 => s.weariness.adjusted = f(),
            24 => s.weariness.enemies.clear(),
            25 => {
                s.weariness.filter = EnemyFilter::AdjustedGetter {
                    allow_pirates: true,
                }
            }
            26 => s.weariness.minimum_for_peace = f(),
            27 => s.priority_rules.max_alignment_modifier = f(),
            28 => s.priority_rules.positive_trait_multiplier = f(),
            29 => s.priority_rules.negative_trait_multiplier = f(),
            30 => s.concern_config.enabled = false,
            31 => s.concern_config.no_auto_generate = true,
            32 => s.concern_config.cooldown_multiplier = f(),
            33 => s.concern_config.anti_repetition_multiplier = f(),
            34 => s.action_configs[0].enabled = false,
            35 => s.action_configs[0].shim = true,
            36 => s.action_configs[0].chance = f(),
            37 => s.action_configs[0].cooldown = f(),
            38 => s.action_configs[0].anti_repetition = f(),
            39 => s.action_configs[0].repetition_id = None,
            40 => s.concerns[1].instance = EntityId::from_bytes([32; 16]),
            41 => s.concerns[0].ended = true,
            42 => s.concerns[0].cooldown = FloatBits::new(0).unwrap(),
            43 => s.concerns[0].action = None,
            44 => s.concerns[0].action.as_mut().unwrap().instance = EntityId::from_bytes([33; 16]),
            45 => s.concerns[0].action.as_mut().unwrap().status = ActionStatus::Failure,
            46 => s.concerns[0].action.as_mut().unwrap().ended = true,
            47 => s.concerns[0].action.as_mut().unwrap().meetings_since_ended += 1,
            48 => s.concerns[0].target_faction = None,
            49 => s.concerns[0].market_id = None,
            50 => s.concerns[0].priority_base = f(),
            51 => s.concerns[0].priority[0].kind = ModifierKind::Percent,
            52 => s.concerns[0].priority[0].value = f(),
            53 => s.timers.meeting += 1,
            54 => s.timers.advance_days = f(),
            55 => s.timers.intervals[0].elapsed = f(),
            56 => s.timers.intervals[0].minimum = f(),
            57 => s.timers.intervals[0].maximum = f(),
            58 => s.timers.draws[0].purpose = "concern-action-selection".into(),
            59 => s.timers.draws[0].value = DoubleBits::new(1).unwrap(),
            60 => s.relations[0].disposition = Some(f()),
            61 => s.action_configs[0].repetition_id = Some("other".into()),
            62 => s.concerns[0].target_faction = Some("hegemony".into()),
            63 => s.concerns[0].market_id = Some("eochu_bres".into()),
            64 => s.concerns[0].priority[0].id = "faction".into(),
            65 => s.timers.intervals[0].id = "strategic-meeting".into(),
            _ => unreachable!(),
        }
        let world =
            NexWorld::admit(s).unwrap_or_else(|e| panic!("mutation {mutation} invalid: {e:?}"));
        assert_ne!(commit_world(&world).unwrap(), digest, "mutation {mutation}");
    }
}
#[test]
fn ordered_arrays_and_unordered_traits_have_explicitly_different_semantics() {
    let original = world_support::rich_snapshot();
    let digest = commit_world(&NexWorld::admit(original.clone()).unwrap()).unwrap();
    for mutation in 0..9 {
        let mut s = original.clone();
        match mutation {
            0 => s.factions.reverse(),
            1 => s.relations.reverse(),
            2 => s.strengths.reverse(),
            3 => s.concern_config.tags.reverse(),
            4 => s.action_configs[0].tags.reverse(),
            5 => s.concerns.reverse(),
            6 => s.concerns[0].priority.reverse(),
            7 => s.timers.intervals.reverse(),
            8 => {
                s.timers.draws.reverse();
                for (i, d) in s.timers.draws.iter_mut().enumerate() {
                    d.ordinal = i as u32;
                }
            }
            _ => unreachable!(),
        }
        assert_ne!(
            commit_world(&NexWorld::admit(s).unwrap()).unwrap(),
            digest,
            "order {mutation}"
        );
    }
    let mut s = original;
    s.factions[0].traits.reverse();
    assert_eq!(commit_world(&NexWorld::admit(s).unwrap()).unwrap(), digest);
}
#[test]
fn identifiers_collection_presence_and_closed_definition_identities_are_covered() {
    let s = world_support::rich_snapshot();
    let original = commit_world(&NexWorld::admit(s.clone()).unwrap()).unwrap();
    for mutation in 0..7 {
        let mut changed = s.clone();
        match mutation {
            0 => {
                changed.factions[1].id = "other".into();
                changed.relations[0].to = "other".into();
                changed.relations[1].from = "other".into();
                changed.strengths[1].faction = "other".into();
                changed.weariness.enemies[0] = "other".into();
                changed.concerns[0].target_faction = Some("other".into());
            }
            1 => {
                changed.strengths[0].markets[0].id = "other_market".into();
                changed.concerns[0].market_id = Some("other_market".into());
            }
            2 => {
                changed.concerns[0].action = None;
                changed.action_configs.clear();
            }
            3 => changed.concerns.pop().map(|_| ()).unwrap(),
            4 => changed.timers.intervals.clear(),
            5 => changed.timers.draws.clear(),
            6 => changed.factions[0].priority_multipliers.clear(),
            _ => unreachable!(),
        }
        assert_ne!(
            commit_world(&NexWorld::admit(changed).unwrap()).unwrap(),
            original,
            "field family {mutation}"
        );
    }
    for mutation in 0..8 {
        let mut invalid = s.clone();
        match mutation {
            0 => invalid.concern_config.definition.id = "unknown".into(),
            1 => invalid.concern_config.definition.class_path = "unknown.Class".into(),
            2 => invalid.action_configs[0].definition.id = "unknown".into(),
            3 => invalid.action_configs[0].definition.module = Module::Executive,
            4 => {
                invalid.concerns[0]
                    .action
                    .as_mut()
                    .unwrap()
                    .definition
                    .class_path = "unknown.Class".into()
            }
            5 => invalid.provenance.runtime_session.0 = 0,
            6 => invalid.provenance.source_digest = [0; 32],
            7 => invalid.timers.draws[0].ordinal = 1,
            _ => unreachable!(),
        }
        assert!(NexWorld::admit(invalid).is_err(), "closed field {mutation}");
    }
}
