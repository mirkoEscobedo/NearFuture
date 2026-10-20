//! Independent fixed-profile bytes and bounded SQL observations for one owned fixture.
use nf_kernel::World;
use nf_store::KnownFrontiers;
use rusqlite::{Connection, types::Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub const PROFILE1_SCHEMA: [u8; 32] = [
    0x7e, 0x06, 0x49, 0x12, 0x04, 0xd7, 0xaf, 0xa6, 0x93, 0x5f, 0x76, 0xf7, 0x69, 0xcb, 0x3f, 0xbf,
    0x98, 0x1c, 0x8b, 0x7c, 0x9c, 0x65, 0x10, 0x06, 0x73, 0xce, 0x3e, 0x32, 0x01, 0x23, 0x6e, 0xbb,
];
pub const PROFILE3_SCHEMA: [u8; 32] = [
    0x8e, 0x35, 0x00, 0xf3, 0x0c, 0xb0, 0x43, 0xe6, 0x8c, 0x15, 0x8b, 0x29, 0xab, 0x9e, 0x11, 0xcd,
    0xd9, 0x7e, 0x7e, 0x31, 0x47, 0x83, 0xc0, 0x18, 0x3b, 0x83, 0x49, 0x9e, 0x32, 0xe5, 0xac, 0x73,
];
pub fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn expected_state(source: &[u8], world: &World) -> Vec<u8> {
    let prefix = b"NF-STORE-1\0\x01\0";
    assert!(source.len() <= 1_048_576);
    assert_eq!(&source[..prefix.len()], prefix);
    let at = prefix.len();
    let size = u32::from_le_bytes(source[at..at + 4].try_into().unwrap()) as usize;
    let tail = at.checked_add(4).unwrap().checked_add(size).unwrap();
    assert!(tail <= source.len());
    let body = nf_kernel::encode_snapshot(world).unwrap();
    let mut expected = prefix.to_vec();
    expected.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
    expected.extend_from_slice(&body);
    expected.extend_from_slice(&source[tail..]);
    expected
}
fn fixture_descriptor(storage: u32, schema: [u8; 32], config: u8) -> Vec<u8> {
    let mut bytes = b"NF-MIGRATION-COMPOSITION-1\0".to_vec();
    bytes.extend_from_slice(&storage.to_le_bytes());
    bytes.extend_from_slice(&schema);
    bytes.extend_from_slice(&[config; 32]);
    bytes.extend_from_slice(&[3; 32]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&[41; 16]);
    bytes.extend_from_slice(&[41; 32]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes
}
pub fn expected_action(
    known: KnownFrontiers,
    head: [u8; 32],
    snapshot: [u8; 32],
    member: [u8; 32],
) -> Vec<u8> {
    let mut bytes = b"NF-STORE-CONFIG-UPGRADE-1".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(known.scope.universe.as_bytes());
    bytes.extend_from_slice(known.scope.history.as_bytes());
    bytes.extend_from_slice(&known.store_revision.to_be_bytes());
    bytes.extend_from_slice(&known.event_sequence.0.to_be_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&known.membership_revision.unwrap().to_be_bytes());
    bytes.extend_from_slice(&head);
    bytes.extend_from_slice(&snapshot);
    bytes.push(1);
    bytes.extend_from_slice(&member);
    for descriptor in [
        fixture_descriptor(1, PROFILE1_SCHEMA, 2),
        fixture_descriptor(3, PROFILE3_SCHEMA, 9),
    ] {
        bytes.extend_from_slice(&u32::try_from(descriptor.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(&descriptor);
    }
    bytes
}
pub fn expected_head(
    previous_head: [u8; 32],
    revision: u64,
    action: &[u8],
    source: [u8; 32],
    target: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"NF-STORE-CONFIG-UPGRADE-1".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&previous_head);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&hash(action));
    bytes.extend_from_slice(&source);
    bytes.extend_from_slice(&target);
    hash(&bytes)
}
pub fn base_tables(connection: &Connection) -> BTreeMap<&'static str, Vec<Vec<Value>>> {
    let mut tables = BTreeMap::new();
    for (table, order) in [
        ("history_meta", "singleton"),
        ("snapshots", "revision"),
        ("journal", "revision"),
        ("events", "sequence"),
        ("aggregate_state", "aggregate_id"),
        ("module_state", "provider_id"),
        ("request_outcomes", "request_id"),
        ("reservations", "operation_id"),
        ("outbox", "operation_id"),
        ("membership", "universe,history"),
    ] {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY {order} LIMIT 16"))
            .unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get::<_, Value>(column))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(
            rows.len() < 16,
            "the tiny owned fixture must not truncate {table}"
        );
        tables.insert(table, rows);
    }
    tables
}
pub struct UpgradeRow {
    pub revision: Vec<u8>,
    pub kind: i32,
    pub previous: Vec<u8>,
    pub digest: Vec<u8>,
    pub head: Vec<u8>,
    pub action: Vec<u8>,
    pub state: Vec<u8>,
}
pub fn upgrade_row(connection: &Connection) -> UpgradeRow {
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM (SELECT 1 FROM config_upgrade_journal LIMIT 2)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    connection
        .query_row(
            "SELECT revision,kind,previous,digest,head,action,state FROM config_upgrade_journal",
            [],
            |r| {
                Ok(UpgradeRow {
                    revision: r.get(0)?,
                    kind: r.get(1)?,
                    previous: r.get(2)?,
                    digest: r.get(3)?,
                    head: r.get(4)?,
                    action: r.get(5)?,
                    state: r.get(6)?,
                })
            },
        )
        .unwrap()
}
pub struct UpgradeMeta {
    pub revision: Vec<u8>,
    pub event: Vec<u8>,
    pub state: Vec<u8>,
    pub digest: Vec<u8>,
    pub head: Vec<u8>,
    pub schema: Vec<u8>,
}
pub fn upgrade_meta(connection: &Connection) -> UpgradeMeta {
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM (SELECT 1 FROM config_upgrade_meta LIMIT 2)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    connection.query_row("SELECT revision,event_seq,state,digest,head,schema_digest FROM config_upgrade_meta WHERE singleton=1", [], |r| {
        Ok(UpgradeMeta { revision: r.get(0)?, event: r.get(1)?, state: r.get(2)?, digest: r.get(3)?, head: r.get(4)?, schema: r.get(5)? })
    }).unwrap()
}
