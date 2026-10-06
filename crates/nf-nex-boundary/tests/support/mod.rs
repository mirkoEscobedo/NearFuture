use nf_contract::identity::{EntityId, EventSeq, HistoryId, RuntimeSession, UniverseId};
use nf_nex_boundary::*;

pub fn number(bits: u32) -> FloatBits {
    FloatBits::new(bits).unwrap()
}
pub fn concern_definition() -> Definition {
    Definition {
        id: "warWeariness".into(),
        class_path: "exerelin.campaign.ai.concern.WarWearinessConcern".into(),
        module: Module::Diplomatic,
    }
}
pub fn snapshot() -> Snapshot {
    Snapshot {
        provenance: Provenance {
            source_commit: SOURCE_COMMIT.into(),
            source_digest: [1; 32],
            runtime_digest: [2; 32],
            ruleset_digest: [3; 32],
            merged_config_digest: [4; 32],
            universe: UniverseId::from_bytes([5; 16]),
            history: HistoryId::from_bytes([6; 16]),
            runtime_session: RuntimeSession(1),
            frontier: EventSeq(7),
            observation: Observation::Synthetic,
        },
        subject_faction: "hegemony".into(),
        factions: vec![FactionView {
            id: "hegemony".into(),
            live: true,
            traits: vec!["stalwart".into()],
            diplomatic_alignment: number(0x3f00_0000),
            priority_multipliers: vec![("warWeariness".into(), number(0x3f80_0000))],
        }],
        relations: vec![],
        strengths: vec![],
        weariness: WearinessView {
            manager_present: true,
            map_entry_present: true,
            raw: number(0x459c_4000),
            adjusted: number(0x459c_4000),
            enemies: vec![],
            filter: EnemyFilter::AdjustedGetter {
                allow_pirates: false,
            },
            minimum_for_peace: number(0x459c_4000),
        },
        priority_rules: PriorityRules {
            max_alignment_modifier: number(0x3f00_0000),
            positive_trait_multiplier: number(0x3fc0_0000),
            negative_trait_multiplier: number(0x3f00_0000),
        },
        concern_config: ConcernConfig {
            definition: concern_definition(),
            enabled: true,
            no_auto_generate: false,
            tags: vec![
                "diplomacy".into(),
                "canMakePeace".into(),
                "trait_pacifist".into(),
                "trait_weak-willed".into(),
                "!trait_stalwart".into(),
            ],
            cooldown_multiplier: number(0x3f80_0000),
            anti_repetition_multiplier: number(0x3f80_0000),
        },
        action_configs: vec![],
        concerns: vec![ConcernView {
            instance: EntityId::from_bytes([9; 16]),
            definition: concern_definition(),
            ended: false,
            cooldown: number(0x8000_0000),
            action: None,
            target_faction: None,
            market_id: None,
            priority_base: number(0),
            priority: vec![Modifier {
                id: "value".into(),
                kind: ModifierKind::Flat,
                value: number(0x4248_0000),
            }],
        }],
        timers: TimerView {
            meeting: 4,
            advance_days: number(0x3e00_0000),
            intervals: vec![],
            draws: vec![],
        },
        extensions: ExtensionInventory {
            complete: true,
            listeners: vec![],
            direct_calls: vec![],
            other_definitions: vec![],
        },
    }
}
