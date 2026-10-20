use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_topology::{Dependency, Entity, Kind, Limits, Topology, decode, digest, encode},
};

fn literal() -> Vec<u8> {
    let hex = include_str!("vectors/system-topology-unsigned-metadata-v1.hex").trim();
    assert_eq!(hex.len(), 398);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn independent_high_bit_unsigned_revision_and_descriptive_date_vector() {
    // NF-SPT-1 defines unsigned64 metadata, regardless of Java's signed long view.
    let expected = Topology {
        universe: UniverseId::from_bytes([0x11; 16]),
        history: HistoryId::from_bytes([0x22; 16]),
        owner: AccountId::from_bytes([0x33; 16]),
        system: EntityId::from_bytes([0x44; 16]),
        revision: 1_u64 << 63,
        campaign_day_millis: u64::MAX,
        dependencies: vec![Dependency {
            semantic_key: [0x77; 32],
            required: true,
        }],
        entities: vec![
            Entity {
                id: EntityId::from_bytes([1; 16]),
                kind: Kind::Star,
                parent: None,
                dependency: None,
                label: Some("Sun".into()),
            },
            Entity {
                id: EntityId::from_bytes([2; 16]),
                kind: Kind::Planet,
                parent: Some(EntityId::from_bytes([1; 16])),
                dependency: Some(0),
                label: Some("World".into()),
            },
        ],
    };
    let bytes = literal();
    assert_eq!(bytes.len(), 199);
    assert_eq!(decode(&bytes, Limits::default()), Ok(expected.clone()));
    assert_eq!(encode(&expected, Limits::default()), Ok(bytes));
    assert_eq!(
        digest(&expected, Limits::default()).unwrap(),
        [
            0x84, 0x0c, 0x17, 0x06, 0xef, 0xbc, 0x0e, 0x08, 0x3b, 0x77, 0x43, 0xc0, 0x1e, 0x99,
            0x64, 0xf6, 0x5b, 0x76, 0x98, 0x76, 0x7e, 0x5b, 0xb5, 0xf9, 0x83, 0xe7, 0xa2, 0x76,
            0x2b, 0x77, 0xfc, 0x53,
        ]
    );
}
