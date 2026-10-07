#![allow(dead_code)]
use nf_nex_boundary::{DoubleBits, FloatBits, Modifier, ModifierKind};
use nf_nex_shadow::*;
pub fn f(bits: &str) -> FloatBits {
    FloatBits::new(u32::from_str_radix(bits, 16).unwrap()).unwrap()
}
pub fn float(v: f32) -> FloatBits {
    FloatBits::new(v.to_bits()).unwrap()
}
pub fn war(fields: &[&str]) -> WarFacts {
    assert_eq!(fields.len(), 13);
    let mask: u8 = fields[6].parse().unwrap();
    assert!(mask < 8);
    let mut traits = vec![];
    for (bit, name) in [(1, "pacifist"), (2, "weak-willed"), (4, "stalwart")] {
        if mask & bit != 0 {
            traits.push(name.into());
        }
    }
    let existing = match fields[5] {
        "none" => vec![],
        "active" => vec![ExistingConcern {
            class: ConcernClass::WarWeariness,
            ended: false,
        }],
        "ended" => vec![ExistingConcern {
            class: ConcernClass::WarWeariness,
            ended: true,
        }],
        "other" => vec![ExistingConcern {
            class: ConcernClass::Other,
            ended: false,
        }],
        _ => panic!("invalid fixture"),
    };
    WarFacts {
        weariness: f(fields[1]),
        minimum: f(fields[2]),
        already_ended: fields[3].parse().unwrap(),
        current_action: fields[4].parse().unwrap(),
        existing,
        priority: PriorityFacts {
            alignment: float(0.5),
            max_alignment: float(0.25),
            positive_trait: float(1.3),
            negative_trait: float(0.7),
            traits,
            faction_multiplier: if fields[7] == "-" {
                None
            } else {
                Some(f(fields[7]))
            },
            tags: [
                "diplomacy",
                "canMakePeace",
                "trait_pacifist",
                "trait_weak-willed",
                "!trait_stalwart",
            ]
            .map(str::to_owned)
            .to_vec(),
            existing: vec![Modifier {
                id: "value".into(),
                kind: ModifierKind::Flat,
                value: float(1.0),
            }],
        },
    }
}
pub fn peace(fields: &[&str]) -> PeaceFacts {
    assert_eq!(fields.len(), 8);
    let gate = fields[1];
    assert!(
        [
            "none",
            "noEnemies",
            "pirate",
            "warInterval",
            "ownWeariness",
            "enemyWeariness",
            "recentWar",
            "canCeasefire",
            "commissioned",
            "offensive"
        ]
        .contains(&gate)
    );
    let mut draws = vec![SuppliedDraw {
        purpose: "peace-chance".into(),
        value: DoubleBits::new(u64::from_str_radix(fields[3], 16).unwrap()).unwrap(),
    }];
    if fields[4] != "-" {
        draws.push(SuppliedDraw {
            purpose: "peace-treaty".into(),
            value: DoubleBits::new(u64::from_str_radix(fields[4], 16).unwrap()).unwrap(),
        });
    }
    PeaceFacts {
        faction: "hegemony".into(),
        enemy: "tritachyon".into(),
        enemy_is_player: false,
        treaty_relation: fields[2].parse().unwrap(),
        own_weariness: if gate == "ownWeariness" {
            f("459c3fff")
        } else {
            float(5000.0)
        },
        enemy_weariness: if gate == "enemyWeariness" {
            f("459c3fff")
        } else {
            float(5000.0)
        },
        own_events: float(0.0),
        enemy_events: float(0.0),
        gates: PeaceGates {
            has_enemies: gate != "noEnemies",
            pirate: gate == "pirate",
            allow_pirate: false,
            days: if gate == "warInterval" {
                f("41efffff")
            } else {
                float(30.0)
            },
            minimum_interval: float(30.0),
            recent_war: gate == "recentWar",
            can_ceasefire: gate != "canCeasefire",
            commissioned: gate == "commissioned",
            offensive_blocked: gate == "offensive",
            offensive_facts_provided: true,
        },
        rules: PeaceRules {
            minimum: float(5000.0),
            divisor: float(20000.0),
            divisor_per_level: float(75.0),
            player_level: 0,
            event_multiplier: float(40.0),
            treaty_chance: float(0.3),
            ceasefire_reduction: float(3000.0),
            treaty_reduction: float(6000.0),
        },
        draws,
    }
}
pub fn modifiers(values: &[Modifier]) -> String {
    if values.is_empty() {
        return "-".into();
    }
    values
        .iter()
        .map(|m| {
            format!(
                "{}:{}:{:08x}",
                m.id,
                match m.kind {
                    ModifierKind::Flat => "FLAT",
                    ModifierKind::Percent => "PERCENT",
                    ModifierKind::Multiplier => "MULTIPLIER",
                },
                m.value.bits()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}
pub fn effects(values: &[EffectCall]) -> String {
    if values.is_empty() {
        return "-".into();
    }
    values
        .iter()
        .map(|v| match v {
            EffectCall::DiplomacyEvent {
                faction,
                enemy,
                event_id,
            } => format!("event:{faction}:{enemy}:{event_id}"),
            EffectCall::Weariness { faction, amount } => {
                format!("weariness:{faction}:{:08x}", amount.bits())
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
