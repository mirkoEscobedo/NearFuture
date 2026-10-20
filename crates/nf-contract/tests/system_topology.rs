use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_topology::*,
};
fn id(n: u8) -> EntityId {
    EntityId::from_bytes([n; 16])
}
fn bytes(hex: &str) -> Vec<u8> {
    hex.trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u8::from_str_radix(core::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn fixture() -> Topology {
    Topology {
        universe: UniverseId::from_bytes([0x11; 16]),
        history: HistoryId::from_bytes([0x22; 16]),
        owner: AccountId::from_bytes([0x33; 16]),
        system: id(0x44),
        revision: 1,
        campaign_day_millis: 2,
        dependencies: vec![Dependency {
            semantic_key: [0x77; 32],
            required: true,
        }],
        entities: vec![
            Entity {
                id: id(1),
                kind: Kind::Star,
                parent: None,
                dependency: None,
                label: Some("Sun".into()),
            },
            Entity {
                id: id(2),
                kind: Kind::Planet,
                parent: Some(id(1)),
                dependency: Some(0),
                label: Some("World".into()),
            },
        ],
    }
}
fn golden() -> Vec<u8> {
    bytes(include_str!("vectors/system-topology-v1.hex"))
}
#[test]
fn public_cycle_refusal_and_independent_acyclic_literal_vector() {
    let mut cycle = fixture();
    cycle.entities[1].parent = Some(id(3));
    cycle.entities.push(Entity {
        id: id(3),
        kind: Kind::Station,
        parent: Some(id(2)),
        dependency: None,
        label: None,
    });
    assert_eq!(validate(&cycle, Limits::default()), Err(Error::Cycle));
    assert_eq!(encode(&cycle, Limits::default()), Err(Error::Cycle));
    assert_eq!(
        decode(
            &bytes(include_str!("vectors/system-topology-cycle-v1.hex")),
            Limits::default()
        ),
        Err(Error::Cycle)
    );
    let literal = golden();
    assert_eq!(literal.len(), 199);
    assert_eq!(decode(&literal, Limits::default()), Ok(fixture()));
    assert_eq!(encode(&fixture(), Limits::default()), Ok(literal));
    assert_eq!(
        digest(&fixture(), Limits::default()).unwrap().as_slice(),
        bytes("f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183")
    );
}
#[test]
fn closed_decoder_refuses_malformed_and_declared_oversize_before_expansion() {
    for (offset, replacement, error) in [
        (9, 2, Error::Invalid),
        (91, 2, Error::Invalid),
        (92, 65, Error::Limit),
        (127, 129, Error::Limit),
        (126, 2, Error::Invalid),
        (192, 129, Error::Limit),
    ] {
        let mut value = golden();
        value[offset] = replacement;
        assert_eq!(decode(&value, Limits::default()), Err(error));
    }
    let mut zero = golden();
    zero[75..83].fill(0);
    assert_eq!(decode(&zero, Limits::default()), Err(Error::Invalid));
    let mut trailing = golden();
    trailing.push(0);
    assert_eq!(decode(&trailing, Limits::default()), Err(Error::Invalid));
    let value = golden();
    assert_eq!(
        decode(&value[..value.len() - 1], Limits::default()),
        Err(Error::Invalid)
    );
    assert_eq!(
        decode(&vec![0; MAX_DOCUMENT_BYTES + 1], Limits::default()),
        Err(Error::Limit)
    );
}
#[test]
fn reference_order_and_text_controls_are_distinct_from_cycle() {
    let mut value = fixture();
    value.entities[1].parent = Some(id(9));
    assert_eq!(validate(&value, Limits::default()), Err(Error::Invalid));
    value = fixture();
    value.entities[1].id = id(1);
    assert_eq!(validate(&value, Limits::default()), Err(Error::Invalid));
    value = fixture();
    value.dependencies.push(value.dependencies[0].clone());
    assert_eq!(validate(&value, Limits::default()), Err(Error::Invalid));
    value = fixture();
    value.entities[1].label = Some("e\u{301}".into());
    assert_eq!(validate(&value, Limits::default()), Err(Error::Invalid));
    value.entities[1].label = Some("x".repeat(128));
    assert_eq!(validate(&value, Limits::default()), Ok(()));
    assert_eq!(
        decode(
            &encode(&value, Limits::default()).unwrap(),
            Limits::default()
        ),
        Ok(value.clone())
    );
    value.entities[1].label = Some("x".repeat(129));
    assert_eq!(validate(&value, Limits::default()), Err(Error::Limit));
    value = fixture();
    value.entities[1].parent = None;
    assert_eq!(validate(&value, Limits::default()), Err(Error::Invalid));
}
#[test]
fn flat_chain128_and_dependency64_accept_but129_and65_refuse() {
    let mut value = fixture();
    value.entities = (1..=128)
        .map(|n| Entity {
            id: id(n),
            kind: if n == 1 { Kind::Star } else { Kind::Planet },
            parent: if n == 1 { None } else { Some(id(n - 1)) },
            dependency: None,
            label: None,
        })
        .collect();
    value.dependencies = (1..=64)
        .map(|n| Dependency {
            semantic_key: [n; 32],
            required: true,
        })
        .collect();
    assert_eq!(validate(&value, Limits::default()), Ok(()));
    assert_eq!(
        decode(
            &encode(&value, Limits::default()).unwrap(),
            Limits::default()
        ),
        Ok(value.clone())
    );
    let lower = Limits {
        entities: 127,
        ..Limits::default()
    };
    assert_eq!(validate(&value, lower), Err(Error::Limit));
    value.entities.push(Entity {
        id: id(129),
        kind: Kind::Planet,
        parent: Some(id(128)),
        dependency: None,
        label: None,
    });
    assert_eq!(validate(&value, Limits::default()), Err(Error::Limit));
    value = fixture();
    value.dependencies = (1..=65)
        .map(|n| Dependency {
            semantic_key: [n; 32],
            required: true,
        })
        .collect();
    assert_eq!(validate(&value, Limits::default()), Err(Error::Limit));
    let zero_label = Limits {
        label_bytes: 0,
        ..Limits::default()
    };
    let mut empty = fixture();
    for entity in &mut empty.entities {
        entity.label = Some(String::new());
    }
    assert_eq!(validate(&empty, zero_label), Ok(()));
    assert_eq!(
        decode(&encode(&empty, zero_label).unwrap(), zero_label),
        Ok(empty.clone())
    );
    empty.entities[0].label = Some("x".into());
    assert_eq!(encode(&empty, zero_label), Err(Error::Limit));
    assert_eq!(
        encode(
            &fixture(),
            Limits {
                label_bytes: 129,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(
        encode(
            &fixture(),
            Limits {
                document_bytes: 198,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
}
#[test]
fn selection_never_imports_hidden_parent_label_or_unselected_content() {
    let value = fixture();
    assert_eq!(
        select(&value, &[id(2)], &[[0x77; 32]], Limits::default()),
        Err(Error::Privacy)
    );
    assert_eq!(
        select(&value, &[id(1), id(2)], &[], Limits::default()),
        Err(Error::Privacy)
    );
    let output = select(&value, &[id(1)], &[], Limits::default()).unwrap();
    assert_eq!(output.entities, vec![value.entities[0].clone()]);
    assert!(output.dependencies.is_empty());
    let encoded = encode(&output, Limits::default()).unwrap();
    assert!(!encoded.windows(5).any(|v| v == b"World"));
    assert_eq!(
        select(&value, &[id(1), id(9)], &[], Limits::default()),
        Err(Error::Privacy)
    );
    assert_eq!(
        select(&value, &[id(1), id(1)], &[], Limits::default()),
        Err(Error::Invalid)
    );
    let mut remap = value.clone();
    remap.dependencies = vec![
        Dependency {
            semantic_key: [0x55; 32],
            required: false,
        },
        value.dependencies[0].clone(),
    ];
    remap.entities[1].dependency = Some(1);
    assert_eq!(
        select(&remap, &[id(1), id(2)], &[[0x77; 32]], Limits::default()),
        Ok(value)
    );
}
#[test]
fn unknown_required_content_is_explicitly_abstract_and_hash_binds_metadata() {
    let value = fixture();
    assert_eq!(
        diagnose_content(&value, &[], Limits::default()),
        Ok(ContentStatus::Abstract(vec![[0x77; 32]]))
    );
    assert_eq!(
        diagnose_content(&value, &[[0x77; 32]], Limits::default()),
        Ok(ContentStatus::Known)
    );
    let mut optional = value.clone();
    optional.dependencies[0].required = false;
    assert_eq!(
        diagnose_content(&optional, &[], Limits::default()),
        Ok(ContentStatus::Known)
    );
    let mut changed = value.clone();
    changed.revision = 2;
    assert_ne!(
        digest(&value, Limits::default()),
        digest(&changed, Limits::default())
    );
    changed = value.clone();
    changed.entities[0].label = Some(String::new());
    let empty = encode(&changed, Limits::default()).unwrap();
    changed.entities[0].label = None;
    assert_ne!(empty, encode(&changed, Limits::default()).unwrap());
}
