#![allow(dead_code)]
#[path = "../../../nf-nex-boundary/tests/support/mod.rs"]
mod boundary_fixture;
use nf_contract::identity::*;
use nf_nex_boundary::*;
use nf_nex_shadow::ShadowMetadata;
pub fn snapshot() -> Snapshot {
    boundary_fixture::snapshot()
}
pub fn metadata(s: &Snapshot) -> ShadowMetadata {
    ShadowMetadata {
        provenance: s.provenance.clone(),
        subject_faction: s.subject_faction.clone(),
        concern_instance: Some(s.concerns[0].instance),
        provider: ProviderId::from_bytes([1; 16]),
        campaign: CampaignId::from_bytes([2; 16]),
        branch: BranchId::from_bytes([3; 16]),
        reference_digest: [1; 32],
        corpus_digest: [2; 32],
        implementation_digest: [3; 32],
        capture_policy_digest: [4; 32],
    }
}
pub fn rich_snapshot() -> Snapshot {
    let mut s = snapshot();
    s.factions[0].traits = vec!["stalwart".into(), "pacifist".into()];
    s.factions.push(FactionView {
        id: "tritachyon".into(),
        live: false,
        traits: vec!["weak-willed".into()],
        diplomatic_alignment: FloatBits::new(0).unwrap(),
        priority_multipliers: vec![],
    });
    s.relations = vec![
        RelationView {
            from: "hegemony".into(),
            to: "tritachyon".into(),
            relationship: FloatBits::new(0xbf00_0000).unwrap(),
            disposition: Some(FloatBits::new(0x8000_0000).unwrap()),
            hostile: true,
        },
        RelationView {
            from: "tritachyon".into(),
            to: "hegemony".into(),
            relationship: FloatBits::new(0x3f00_0000).unwrap(),
            disposition: None,
            hostile: false,
        },
    ];
    s.strengths = vec![
        StrengthView {
            faction: "hegemony".into(),
            markets: vec![MarketStrength {
                id: "jangala".into(),
                size: 5,
                strength: FloatBits::new(0x42c8_0000).unwrap(),
            }],
            fleet_strength: FloatBits::new(0x4120_0000).unwrap(),
        },
        StrengthView {
            faction: "tritachyon".into(),
            markets: vec![MarketStrength {
                id: "eochu_bres".into(),
                size: 3,
                strength: FloatBits::new(0x4248_0000).unwrap(),
            }],
            fleet_strength: FloatBits::new(0x40a0_0000).unwrap(),
        },
    ];
    s.weariness.enemies = vec!["tritachyon".into()];
    let d = Definition {
        id: "makePeace".into(),
        class_path: "exerelin.campaign.ai.action.MakePeaceAction".into(),
        module: Module::Diplomatic,
    };
    s.action_configs = vec![ActionConfig {
        definition: d.clone(),
        enabled: true,
        shim: false,
        tags: vec!["diplomacy".into(), "canMakePeace".into(), "friendly".into()],
        chance: FloatBits::new(0x3f00_0000).unwrap(),
        cooldown: FloatBits::new(0x3f80_0000).unwrap(),
        anti_repetition: FloatBits::new(0x4000_0000).unwrap(),
        repetition_id: Some("peace".into()),
    }];
    s.concerns[0].action = Some(ActionView {
        instance: EntityId::from_bytes([8; 16]),
        definition: d,
        status: ActionStatus::InProgress,
        ended: false,
        meetings_since_ended: 2,
    });
    s.concerns[0].target_faction = Some("tritachyon".into());
    s.concerns[0].market_id = Some("jangala".into());
    s.concerns[0].priority.push(Modifier {
        id: "alignment_diplomatic".into(),
        kind: ModifierKind::Multiplier,
        value: FloatBits::new(0x3fa0_0000).unwrap(),
    });
    let mut second = s.concerns[0].clone();
    second.instance = EntityId::from_bytes([10; 16]);
    second.ended = true;
    second.action = None;
    second.target_faction = None;
    second.market_id = None;
    s.concerns.push(second);
    s.timers.intervals = vec![
        IntervalView {
            id: "strategic-short".into(),
            elapsed: FloatBits::new(0x3f00_0000).unwrap(),
            minimum: FloatBits::new(0x3f80_0000).unwrap(),
            maximum: FloatBits::new(0x4000_0000).unwrap(),
        },
        IntervalView {
            id: "diplomacy-brain".into(),
            elapsed: FloatBits::new(0).unwrap(),
            minimum: FloatBits::new(0).unwrap(),
            maximum: FloatBits::new(0x3f80_0000).unwrap(),
        },
    ];
    s.timers.draws = vec![
        RandomDraw {
            purpose: "peace-choice".into(),
            ordinal: 0,
            value: DoubleBits::new(0x3fd0_0000_0000_0000).unwrap(),
        },
        RandomDraw {
            purpose: "diplomacy-interval".into(),
            ordinal: 1,
            value: DoubleBits::new(0x3fe0_0000_0000_0000).unwrap(),
        },
    ];
    s
}
