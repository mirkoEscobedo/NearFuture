use nf_nex_boundary::{BoundaryError, Definition, Module, classify_definition};

#[test]
fn replaced_class_path_has_explicit_unsupported_outcome() {
    let definition = Definition {
        id: "warWeariness".into(),
        class_path: "third.party.ReplacedConcern".into(),
        module: Module::Diplomatic,
    };
    assert_eq!(
        classify_definition(&definition),
        Err(BoundaryError::UnknownDefinition {
            id: "warWeariness".into(),
            class_path: "third.party.ReplacedConcern".into(),
        })
    );
}

#[test]
fn public_definition_diagnostics_are_bounded_before_copying_inputs() {
    let definition = Definition {
        id: "x".repeat(257),
        class_path: "third.party.C".into(),
        module: Module::Diplomatic,
    };
    assert!(matches!(
        classify_definition(&definition),
        Err(BoundaryError::Limit)
    ));
}
