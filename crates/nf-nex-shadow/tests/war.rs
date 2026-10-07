use nf_nex_boundary::{FloatBits, Modifier, ModifierKind};
use nf_nex_shadow::{
    ConcernClass, ExistingConcern, PriorityFacts, WarFacts, WarOperation, evaluate_war,
};
fn float(bits: u32) -> FloatBits {
    FloatBits::new(bits).unwrap()
}
fn facts(weariness: u32) -> WarFacts {
    WarFacts {
        weariness: float(weariness),
        minimum: float(0x459c4000),
        already_ended: false,
        current_action: true,
        existing: vec![],
        priority: PriorityFacts {
            alignment: float(0x3f000000),
            max_alignment: float(0x3e800000),
            positive_trait: float(0x3fa66666),
            negative_trait: float(0x3f333333),
            traits: vec![],
            faction_multiplier: None,
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
                value: float(0x3f800000),
            }],
        },
    }
}
#[test]
fn threshold_lifecycle_priority_and_exact_class_suppression() {
    let below = evaluate_war(&facts(0x456a5fff), WarOperation::Generate).unwrap();
    assert!(
        !below.generated
            && below.ended
            && below.abort_current_action
            && !below.valid
            && below.writes.is_empty()
    );
    let equal = evaluate_war(&facts(0x456a6000), WarOperation::Generate).unwrap();
    assert!(equal.generated && !equal.ended && !equal.abort_current_action && equal.valid);
    assert_eq!(equal.writes[0].value.bits(), 0x42160000);
    assert_eq!(equal.writes[1].value.bits(), 0x3f900000);
    assert_eq!(equal.existing_priority, facts(0x456a6000).priority.existing);
    let mut active = facts(0x459c4000);
    active.existing.push(ExistingConcern {
        class: ConcernClass::WarWeariness,
        ended: false,
    });
    let suppressed = evaluate_war(&active, WarOperation::Generate).unwrap();
    assert!(!suppressed.generated && suppressed.writes.is_empty());
    active.existing[0].class = ConcernClass::Other;
    assert!(
        evaluate_war(&active, WarOperation::Generate)
            .unwrap()
            .generated
    );
    active.already_ended = true;
    assert!(
        !evaluate_war(&active, WarOperation::Generate)
            .unwrap()
            .generated
    );
    assert_eq!(
        evaluate_war(&active, WarOperation::Update)
            .unwrap()
            .writes
            .len(),
        2
    );
}
