use nf_nex_boundary::{DoubleBits, FloatBits};
use nf_nex_shadow::{
    PeaceDecision, PeaceFacts, PeaceGates, PeaceRules, SuppliedDraw, Unavailable,
    evaluate_selected_peace,
};
fn f(v: f32) -> FloatBits {
    FloatBits::new(v.to_bits()).unwrap()
}
fn input(draw: u64) -> PeaceFacts {
    PeaceFacts {
        faction: "hegemony".into(),
        enemy: "tritachyon".into(),
        enemy_is_player: false,
        treaty_relation: false,
        own_weariness: f(5000.0),
        enemy_weariness: f(5000.0),
        own_events: f(0.0),
        enemy_events: f(0.0),
        gates: PeaceGates {
            has_enemies: true,
            pirate: false,
            allow_pirate: false,
            days: f(30.0),
            minimum_interval: f(30.0),
            recent_war: false,
            can_ceasefire: true,
            commissioned: false,
            offensive_blocked: false,
            offensive_facts_provided: true,
        },
        rules: PeaceRules {
            minimum: f(5000.0),
            divisor: f(20000.0),
            divisor_per_level: f(75.0),
            player_level: 0,
            event_multiplier: f(40.0),
            treaty_chance: f(0.3),
            ceasefire_reduction: f(3000.0),
            treaty_reduction: f(6000.0),
        },
        draws: vec![SuppliedDraw {
            purpose: "peace-chance".into(),
            value: DoubleBits::new(draw).unwrap(),
        }],
    }
}
#[test]
fn double_boundaries_draw_consumption_and_proposal_effect_order() {
    let equal = evaluate_selected_peace(&input(0x3fe0000000000000)).unwrap();
    assert_eq!(equal.decision, PeaceDecision::Ceasefire);
    assert_eq!(equal.consumed_draws, 1);
    assert_eq!(equal.effects.len(), 3);
    assert_eq!(
        evaluate_selected_peace(&input(0x3fe0000000000001))
            .unwrap()
            .decision,
        PeaceDecision::None
    );
    let mut treaty = input(0x3fe0000000000000);
    treaty.treaty_relation = true;
    assert_eq!(
        evaluate_selected_peace(&treaty),
        Err(Unavailable::MissingFact)
    );
    treaty.draws.push(SuppliedDraw {
        purpose: "peace-treaty".into(),
        value: DoubleBits::new(0x3fd333333fffffff).unwrap(),
    });
    assert_eq!(
        evaluate_selected_peace(&treaty).unwrap().decision,
        PeaceDecision::Treaty
    );
    treaty.draws[1].value = DoubleBits::new(0x3fd3333340000000).unwrap();
    assert_eq!(
        evaluate_selected_peace(&treaty).unwrap().decision,
        PeaceDecision::Ceasefire
    );
    treaty.enemy_is_player = true;
    assert_eq!(
        evaluate_selected_peace(&treaty),
        Err(Unavailable::Unsupported)
    );
}
