//! Independent fixed READ_ONLY SQL capture, separate from the future StrategicReader.
use super::{Case, hash};
use nf_store::KnownFrontiers;
use rusqlite::{
    Connection, DatabaseName, OpenFlags,
    limits::Limit,
    types::{Value, ValueRef},
};
use std::{collections::BTreeMap, path::Path, time::Duration};
#[derive(Clone, PartialEq)]
pub struct SqlImage {
    rows: BTreeMap<&'static str, Vec<Vec<Value>>>,
}
const QUERIES: [(&str, &str, usize); 10] = [
    (
        "history_meta",
        "SELECT * FROM history_meta ORDER BY singleton LIMIT 2",
        1,
    ),
    (
        "snapshots",
        "SELECT * FROM snapshots ORDER BY revision LIMIT 2",
        1,
    ),
    (
        "journal",
        "SELECT * FROM journal ORDER BY revision LIMIT 5",
        4,
    ),
    (
        "events",
        "SELECT * FROM events ORDER BY sequence LIMIT 3",
        2,
    ),
    (
        "aggregate_state",
        "SELECT * FROM aggregate_state ORDER BY aggregate_id LIMIT 4",
        3,
    ),
    (
        "module_state",
        "SELECT * FROM module_state ORDER BY provider_id LIMIT 2",
        1,
    ),
    (
        "request_outcomes",
        "SELECT * FROM request_outcomes ORDER BY request_id LIMIT 3",
        2,
    ),
    (
        "reservations",
        "SELECT * FROM reservations ORDER BY operation_id LIMIT 1",
        0,
    ),
    (
        "outbox",
        "SELECT * FROM outbox ORDER BY operation_id LIMIT 3",
        2,
    ),
    (
        "membership",
        "SELECT * FROM membership ORDER BY universe,history LIMIT 2",
        1,
    ),
];
fn value(case: &Case, row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Value> {
    match case.guard(|| row.get_ref(column))? {
        ValueRef::Null => Ok(Value::Null),
        ValueRef::Integer(v) => Ok(Value::Integer(v)),
        ValueRef::Blob(bytes) if bytes.len() <= 1_048_576 => Ok(Value::Blob(bytes.to_vec())),
        // These ten schema tables have no text/real cells.
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn blob(value: &Value) -> &[u8] {
    match value {
        Value::Blob(bytes) => bytes,
        _ => panic!("expected fixture blob"),
    }
}
fn integer(value: &Value) -> i64 {
    match value {
        Value::Integer(v) => *v,
        _ => panic!("expected fixture integer"),
    }
}
impl SqlImage {
    pub fn capture(case: &Case, path: &Path) -> Self {
        case.check();
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = path.as_os_str().to_os_string();
            sidecar.push(suffix);
            match case.guard(|| std::fs::symlink_metadata(Path::new(&sidecar))) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => panic!("quiescent fixture has a companion"),
            }
        }
        let connection = case
            .guard(|| {
                Connection::open_with_flags(
                    path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
            })
            .unwrap();
        case.guard(|| connection.busy_timeout(Duration::from_millis(250)))
            .unwrap();
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152));
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536));
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64));
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128));
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 64));
        case.guard(|| connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0));
        for pragma in [
            "PRAGMA query_only=ON;",
            "PRAGMA trusted_schema=OFF;",
            "PRAGMA cache_size=-2048;",
        ] {
            case.guard(|| connection.execute_batch(pragma)).unwrap();
        }
        assert!(
            case.guard(|| connection.is_readonly(DatabaseName::Main))
                .unwrap()
        );
        let query_only: i32 = case
            .guard(|| {
                connection.pragma_query_value(None, "query_only", |r| case.guard(|| r.get(0)))
            })
            .unwrap();
        let trusted: i32 = case
            .guard(|| {
                connection.pragma_query_value(None, "trusted_schema", |r| case.guard(|| r.get(0)))
            })
            .unwrap();
        assert_eq!((query_only, trusted), (1, 0));
        let app: i32 = case
            .guard(|| {
                connection.pragma_query_value(None, "application_id", |r| case.guard(|| r.get(0)))
            })
            .unwrap();
        let version: i32 = case
            .guard(|| {
                connection.pragma_query_value(None, "user_version", |r| case.guard(|| r.get(0)))
            })
            .unwrap();
        assert_eq!((app, version), (0x4e46_5354, 1));
        let mut result = BTreeMap::new();
        for (name, query, cap) in QUERIES {
            case.check();
            let mut statement = case.guard(|| connection.prepare(query)).unwrap();
            let columns = case.guard(|| statement.column_count());
            assert!(columns <= 10);
            // This result borrows the mutable statement; explicit same-clock boundaries.
            case.check();
            let cursor_result = statement.query_map([], |row| {
                (0..columns)
                    .map(|i| value(case, row, i))
                    .collect::<rusqlite::Result<Vec<_>>>()
            });
            case.check();
            let mut cursor = cursor_result.unwrap();
            let mut rows = Vec::new();
            while let Some(row) = case.guard(|| cursor.next()) {
                rows.push(row.unwrap());
            }
            case.guard(|| drop(cursor));
            case.guard(|| drop(statement));
            assert!(rows.len() <= cap);
            assert!(result.insert(name, rows).is_none());
            case.check();
        }
        case.guard(|| connection.close())
            .map_err(|(_, error)| error)
            .unwrap();
        case.check();
        Self { rows: result }
    }
    pub fn assert_fixture(
        &self,
        known: KnownFrontiers,
        canonical: &[u8],
        member: &[u8],
        pending: bool,
    ) {
        let meta = &self.rows["history_meta"];
        assert_eq!(meta.len(), 1);
        let m = &meta[0];
        assert_eq!(integer(&m[0]), 1);
        assert!(blob(&m[1]) == known.scope.universe.as_bytes());
        assert!(blob(&m[2]) == known.scope.history.as_bytes());
        assert_eq!(blob(&m[3]), known.store_revision.to_be_bytes());
        assert_eq!(blob(&m[4]), known.event_sequence.0.to_be_bytes());
        assert!(
            blob(&m[5]) == canonical,
            "independently held canonical body"
        );
        assert_eq!(blob(&m[6]), hash(canonical));
        assert_eq!(blob(&m[7]).len(), 32);
        assert_eq!(blob(&m[8]), PROFILE1_SCHEMA_SHA);
        let rows = &self.rows["membership"];
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert!(blob(&r[0]) == known.scope.universe.as_bytes());
        assert!(blob(&r[1]) == known.scope.history.as_bytes());
        assert_eq!(
            blob(&r[2]),
            known.membership_revision.unwrap().to_be_bytes()
        );
        assert!(
            blob(&r[3]) == member,
            "independent RAM public-membership codec"
        );
        assert_eq!(blob(&r[4]), hash(member));
        let decoded = nf_identity::codec::decode_state(member).unwrap();
        assert_eq!(decoded.accounts.len(), 1);
        assert_eq!(decoded.devices.len(), 1);
        assert_eq!(decoded.revision, known.membership_revision.unwrap());
        assert_eq!(decoded.scope, known.scope);
        assert_eq!(self.rows["snapshots"].len(), 1);
        assert_eq!(self.rows["journal"].len(), known.store_revision as usize);
        assert_eq!(self.rows["events"].len(), known.event_sequence.0 as usize);
        assert_eq!(self.rows["aggregate_state"].len(), 3);
        assert_eq!(self.rows["module_state"].len(), 1);
        assert_eq!(self.rows["request_outcomes"].len(), 2);
        assert_eq!(self.rows["reservations"].len(), 0);
        assert_eq!(self.rows["outbox"].len(), if pending { 1 } else { 2 });
        let requests = &self.rows["request_outcomes"];
        assert_eq!(integer(&requests[0][4]), 2);
        assert_eq!(integer(&requests[1][4]), if pending { 1 } else { 2 });
        assert_eq!(blob(&requests[0][0]), [7; 16]);
        assert_eq!(blob(&requests[1][0]), [8; 16]);
        assert_eq!(blob(&requests[0][3]), [7; 16]);
        assert_eq!(blob(&requests[1][3]), [8; 16]);
        for row in &self.rows["outbox"] {
            assert_eq!(integer(&row[3]), 0);
            assert_eq!(blob(&row[2]).len(), 32);
        }
    }
}
const PROFILE1_SCHEMA_SHA: [u8; 32] = [
    0x7e, 0x06, 0x49, 0x12, 0x04, 0xd7, 0xaf, 0xa6, 0x93, 0x5f, 0x76, 0xf7, 0x69, 0xcb, 0x3f, 0xbf,
    0x98, 0x1c, 0x8b, 0x7c, 0x9c, 0x65, 0x10, 0x06, 0x73, 0xce, 0x3e, 0x32, 0x01, 0x23, 0x6e, 0xbb,
];
