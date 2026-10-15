//! Read-only observations after actual owner handles release their exclusive connections.
use super::records::{PROFILE4_SCHEMA, blob, hash};
use rusqlite::{Connection, OpenFlags, types::Value};
use std::{collections::BTreeMap, path::Path};
pub const ACTIVE_SCHEMA: &str = r#"
CREATE TABLE history_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), state BLOB NOT NULL CHECK(length(state)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;
CREATE TABLE snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32)) STRICT;
CREATE TABLE journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 3), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=1048576), state BLOB NOT NULL CHECK(length(state)<=1048576)) STRICT;
CREATE TABLE events (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), body BLOB NOT NULL CHECK(length(body)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE aggregate_state (aggregate_id BLOB PRIMARY KEY CHECK(length(aggregate_id)=16), revision BLOB NOT NULL CHECK(length(revision)=8)) STRICT;
CREATE TABLE module_state (provider_id BLOB PRIMARY KEY CHECK(length(provider_id)=16), draws BLOB NOT NULL CHECK(length(draws)=8), cooldown BLOB NOT NULL CHECK(length(cooldown)=8)) STRICT;
CREATE TABLE request_outcomes (request_id BLOB PRIMARY KEY CHECK(length(request_id)=16), binding_digest BLOB NOT NULL CHECK(length(binding_digest)=32), intent_digest BLOB NOT NULL CHECK(length(intent_digest)=32), operation_id BLOB NOT NULL CHECK(length(operation_id)=16), phase INTEGER NOT NULL CHECK(phase IN (1,2)), event_seq BLOB CHECK(event_seq IS NULL OR length(event_seq)=8), rejection INTEGER CHECK(rejection IS NULL OR rejection BETWEEN 0 AND 17)) STRICT;
CREATE TABLE reservations (operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16), market_id BLOB NOT NULL CHECK(length(market_id)=16), amount BLOB NOT NULL CHECK(length(amount)=8)) STRICT;
CREATE TABLE outbox (operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16), sequence BLOB NOT NULL CHECK(length(sequence)=8), batch_digest BLOB NOT NULL CHECK(length(batch_digest)=32), rejection INTEGER NOT NULL CHECK(rejection BETWEEN 0 AND 17)) STRICT;
CREATE TABLE membership (universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), public_state BLOB NOT NULL CHECK(length(public_state)<=262144), digest BLOB NOT NULL CHECK(length(digest)=32), PRIMARY KEY(universe,history)) STRICT;
CREATE TABLE active_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), membership_revision BLOB CHECK(membership_revision IS NULL OR length(membership_revision)=8), quota BLOB NOT NULL CHECK(length(quota)=4), state BLOB NOT NULL CHECK(length(state)<=1884160), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), lineage_digest BLOB NOT NULL CHECK(length(lineage_digest)=32), checkpoint_revision BLOB NOT NULL CHECK(length(checkpoint_revision)=8), checkpoint_digest BLOB NOT NULL CHECK(length(checkpoint_digest)=32), checkpoint_head BLOB NOT NULL CHECK(length(checkpoint_head)=32)) STRICT;
CREATE TABLE active_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 7), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=1114112), state BLOB NOT NULL CHECK(length(state)<=1884160)) STRICT;
CREATE TABLE active_events (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), revision BLOB NOT NULL UNIQUE CHECK(length(revision)=8), body BLOB NOT NULL CHECK(length(body)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE active_snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), membership_revision BLOB CHECK(membership_revision IS NULL OR length(membership_revision)=8), quota BLOB NOT NULL CHECK(length(quota)=4), state BLOB NOT NULL CHECK(length(state)<=1884160), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), lineage_digest BLOB NOT NULL CHECK(length(lineage_digest)=32)) STRICT;
CREATE TABLE migration_lineage (singleton INTEGER PRIMARY KEY CHECK(singleton=1), record BLOB NOT NULL CHECK(length(record)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
"#;

pub type Rows = BTreeMap<String, Vec<Vec<Value>>>;
pub(super) fn readonly_public(path: &Path) -> Connection { readonly(path) }
fn readonly(path: &Path) -> Connection {
    let _admitted_file=super::fixture::bounded_read(path);
    for suffix in ["-wal", "-shm", "-journal"] {
        let companion = std::path::PathBuf::from(format!("{}{}", path.display(), suffix));
        assert!(matches!(std::fs::symlink_metadata(companion), Err(e) if e.kind() == std::io::ErrorKind::NotFound));
    }
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}
pub(super) fn tables(connection: &Connection, active: bool) -> Rows {
    let mut result = Rows::new();
    let mut names = vec![
        ("history_meta", "singleton"), ("snapshots", "revision"), ("journal", "revision"),
        ("events", "sequence"), ("aggregate_state", "aggregate_id"), ("module_state", "provider_id"),
        ("request_outcomes", "request_id"), ("reservations", "operation_id"), ("outbox", "operation_id"),
        ("membership", "universe,history"),
    ];
    if active {
        names.extend([
            ("active_meta", "singleton"), ("active_journal", "revision"),
            ("active_events", "sequence"), ("active_snapshots", "revision"),
            ("migration_lineage", "singleton"),
        ]);
    }
    for (table, order) in names {
        let mut statement = connection.prepare(&format!("SELECT * FROM {table} ORDER BY {order} LIMIT 16")).unwrap();
        let columns = statement.column_count();
        assert!(columns <= 64);
        let rows = statement.query_map([], |row| {
            (0..columns).map(|column| row.get::<_, Value>(column))
                .collect::<rusqlite::Result<Vec<_>>>()
        }).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        assert!(rows.len() < 16, "tiny fixture row truncation: {table}");
        result.insert(table.to_string(), rows);
    }
    result
}
pub fn base_rows(path: &Path) -> Rows { tables(&readonly(path), false) }
pub fn rows_hash(rows: &Rows) -> [u8; 32] {
    let mut bytes = b"NF-OWNER-ROWS-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(rows.len()).unwrap().to_le_bytes());
    for (table, rows) in rows {
        blob(&mut bytes, table.as_bytes());
        let columns = match table.as_str() {
            "history_meta" => 9, "snapshots" => 4, "journal" => 7, "events" => 3,
            "aggregate_state" => 2, "module_state" => 3, "request_outcomes" => 7,
            "reservations" => 3, "outbox" => 4, "membership" => 5,
            "active_meta" => 13, "active_journal" => 7, "active_events" => 4,
            "active_snapshots" => 8, "migration_lineage" => 3,
            "owner_retirement" => 3, "artifact_registry" => 6, "operation_receipts" => 4,
            _ => panic!("unknown selected table"),
        };
        bytes.extend_from_slice(&u32::try_from(columns).unwrap().to_le_bytes());
        bytes.extend_from_slice(&u64::try_from(rows.len()).unwrap().to_be_bytes());
        for row in rows {
            assert_eq!(row.len(), columns);
            for cell in row {
                match cell {
                    Value::Null => bytes.push(0),
                    Value::Integer(value) => {
                        bytes.push(1); bytes.extend_from_slice(&value.to_le_bytes());
                    }
                    Value::Blob(value) => { bytes.push(2); blob(&mut bytes, value); }
                    _ => panic!("unexpected REAL/TEXT cell"),
                }
            }
        }
    }
    hash(&bytes)
}
pub fn source_head(path: &Path) -> [u8; 32] {
    readonly(path).query_row("SELECT head FROM history_meta WHERE singleton=1", [], |row| {
        row.get::<_, Vec<u8>>(0)
    }).unwrap().try_into().unwrap()
}
pub(super) fn value(row: &[Value], index: usize) -> &[u8] {
    match &row[index] { Value::Blob(bytes) => bytes, _ => panic!("expected BLOB") }
}
fn option_counter(bytes: &mut Vec<u8>, cell: &Value) {
    match cell {
        Value::Null => bytes.push(0),
        Value::Blob(value) => { assert_eq!(value.len(), 8); bytes.push(1); bytes.extend_from_slice(value); }
        _ => panic!("counter type"),
    }
}
fn assert_schema(connection: &Connection) {
    assert_eq!(connection.pragma_query_value::<i32, _>(None, "application_id", |r| r.get(0)).unwrap(), 0x4e46_5354);
    assert_eq!(connection.pragma_query_value::<i32, _>(None, "user_version", |r| r.get(0)).unwrap(), 4);
    assert_eq!(hash(ACTIVE_SCHEMA.as_bytes()), PROFILE4_SCHEMA);
    let mut expected = BTreeMap::new();
    for line in ACTIVE_SCHEMA.lines().filter(|line| line.starts_with("CREATE TABLE ")) {
        let name = line.split_whitespace().nth(2).unwrap();
        expected.insert(name.to_string(), line.trim_end_matches(';').to_string());
    }
    let mut statement = connection.prepare(
        "SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 16",
    ).unwrap();
    let actual = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).unwrap().collect::<rusqlite::Result<BTreeMap<_, _>>>().unwrap();
    assert_eq!(actual, expected);
    let extra: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' AND type!='table' AND sql IS NOT NULL",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(extra, 0);
}
pub fn active_foot(path: &Path) -> Vec<u8> {
    let connection = readonly(path);
    assert_schema(&connection);
    let rows = tables(&connection, true);
    let meta = &rows["active_meta"][0];
    let checkpoint = rows["active_snapshots"].iter()
        .find(|row| value(row, 0) == value(meta, 10)).unwrap();
    let state = value(meta, 5);
    let offset = b"NF-ACTIVE-STATE-1\0".len() + 2;
    let size = u32::from_le_bytes(state[offset..offset + 4].try_into().unwrap()) as usize;
    let core = &state[offset + 4..offset + 4 + size];
    let member_at = offset + 4 + size;
    assert_eq!(state[member_at], 1);
    let length = u32::from_le_bytes(state[member_at + 1..member_at + 5].try_into().unwrap()) as usize;
    let member = &state[member_at + 5..member_at + 5 + length];
    let tail = member_at + 5 + length;
    assert_eq!(&state[tail..], &[0, 0, 1, 0, 0, 0, 0, 0, 0]);
    let base = &rows["history_meta"][0];
    let mut foot = b"NF-OWNER-STORAGE-1\0".to_vec();
    foot.extend_from_slice(&1u16.to_le_bytes());
    foot.extend_from_slice(&4u32.to_le_bytes());
    foot.extend_from_slice(&PROFILE4_SCHEMA);
    foot.extend_from_slice(value(base, 1));
    foot.extend_from_slice(value(base, 2));
    foot.extend_from_slice(value(meta, 1));
    foot.extend_from_slice(value(meta, 2));
    option_counter(&mut foot, &meta[3]);
    foot.extend_from_slice(&hash(core));
    foot.push(1); foot.extend_from_slice(&hash(member));
    foot.extend_from_slice(&hash(state));
    foot.extend_from_slice(value(meta, 7));
    foot.push(1); foot.extend_from_slice(value(meta, 9));
    foot.extend_from_slice(value(meta, 4));
    foot.push(0);
    foot.extend_from_slice(&hash(&0u32.to_le_bytes()));
    foot.extend_from_slice(value(checkpoint, 0));
    foot.extend_from_slice(value(checkpoint, 1));
    option_counter(&mut foot, &checkpoint[2]);
    foot.extend_from_slice(value(checkpoint, 5));
    foot.extend_from_slice(value(checkpoint, 6));
    foot.extend_from_slice(value(checkpoint, 3));
    foot.extend_from_slice(&rows_hash(&rows));
    foot
}
pub fn source_foot(base: &Rows, member: &nf_identity::model::MembershipState) -> Vec<u8> {
    let meta = &base["history_meta"][0];
    let checkpoint = &base["snapshots"][0];
    let core = value(meta, 5);
    let checkpoint_core = value(checkpoint, 1);
    let world_at = b"NF-STORE-1\0\x01\0".len();
    let size = u32::from_le_bytes(checkpoint_core[world_at..world_at+4].try_into().unwrap()) as usize;
    let world = nf_kernel::decode_snapshot(&checkpoint_core[world_at+4..world_at+4+size]).unwrap();
    // Explicit current-member/Q projection, not a claim of legacy durable checkpoint membership.
    let checkpoint_envelope = super::records::envelope(checkpoint_core, member, None);
    let state = super::records::envelope(core, member, None);
    let mut out = b"NF-OWNER-STORAGE-1\0".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&super::records::PROFILE1_SCHEMA);
    out.extend_from_slice(value(meta, 1)); out.extend_from_slice(value(meta, 2));
    out.extend_from_slice(value(meta, 3)); out.extend_from_slice(value(meta, 4));
    out.push(1); out.extend_from_slice(&member.revision.to_be_bytes());
    out.extend_from_slice(&hash(core));
    out.push(1); out.extend_from_slice(&hash(&nf_identity::codec::encode_state(member).unwrap()));
    out.extend_from_slice(&hash(&state)); out.extend_from_slice(value(meta, 7));
    out.push(0); out.extend_from_slice(&65_536u32.to_le_bytes());
    out.push(0); out.extend_from_slice(&hash(&0u32.to_le_bytes()));
    out.extend_from_slice(value(checkpoint, 0));
    out.extend_from_slice(&world.view().event_seq().0.to_be_bytes());
    out.push(1); out.extend_from_slice(&member.revision.to_be_bytes());
    out.extend_from_slice(&hash(&checkpoint_envelope));
    out.extend_from_slice(value(checkpoint, 3));
    out.extend_from_slice(&65_536u32.to_le_bytes());
    out.extend_from_slice(&rows_hash(base));
    out
}
