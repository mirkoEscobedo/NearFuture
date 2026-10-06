mod support;
use nf_nex_boundary::*;

#[test]
fn extension_inventory_is_enumerated_and_cannot_become_authority() {
    let mut snapshot = support::snapshot();
    snapshot.extensions.complete = false;
    snapshot.extensions.listeners.push(Registration {
        class_path: "third.party.Listener".into(),
        origin: "mod-a".into(),
    });
    snapshot.extensions.direct_calls.push(Registration {
        class_path: "third.party.PeaceCaller".into(),
        origin: "mod-b".into(),
    });
    snapshot.extensions.other_definitions.push(Definition {
        id: "otherConcern".into(),
        class_path: "third.party.Concern".into(),
        module: Module::Diplomatic,
    });
    let assessment = assess_extensions(&snapshot.extensions).unwrap();
    assert_eq!(assessment.authority, Authority::ShadowOnly);
    assert_eq!(assessment.runtime, RuntimeCertification::Unobserved);
    assert_eq!(
        assessment.findings,
        vec![
            CompatibilityFinding::IncompleteInventory,
            CompatibilityFinding::UncharacterizedListener {
                class_path: "third.party.Listener".into(),
                origin: "mod-a".into()
            },
            CompatibilityFinding::UncharacterizedDirectCall {
                class_path: "third.party.PeaceCaller".into(),
                origin: "mod-b".into()
            },
            CompatibilityFinding::LegacyOnlyDefinition {
                id: "otherConcern".into(),
                class_path: "third.party.Concern".into()
            },
        ]
    );
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::InvalidFact("extension inventory"))
    );
}

#[test]
fn adversarial_inventory_cannot_amplify_unbounded_strings() {
    let mut inventory = support::snapshot().extensions;
    inventory.listeners = vec![Registration {
        class_path: "x".repeat(257),
        origin: "mod-a".into(),
    }];
    assert_eq!(assess_extensions(&inventory), Err(BoundaryError::Limit));
}
