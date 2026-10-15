//! Exact physical DDL, including absence of extra user objects.
use rusqlite::Connection;
use std::collections::BTreeMap;
pub fn check(c:&Connection,app:i32,version:i32,schema:&str,digest:[u8;32]){
    assert_eq!(super::records::hash(schema.as_bytes()),digest);
    assert_eq!(c.pragma_query_value::<i32,_>(None,"application_id",|r|r.get(0)).unwrap(),app);
    assert_eq!(c.pragma_query_value::<i32,_>(None,"user_version",|r|r.get(0)).unwrap(),version);
    assert_eq!(c.pragma_query_value::<u32,_>(None,"page_size",|r|r.get(0)).unwrap(),4096);
    let expected=schema.lines().filter(|line|line.starts_with("CREATE TABLE ")).map(|line|
        (line.split_whitespace().nth(2).unwrap().to_string(),line.trim_end_matches(';').to_string()))
        .collect::<BTreeMap<_,_>>();
    let actual=c.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 17").unwrap()
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).unwrap()
        .collect::<rusqlite::Result<BTreeMap<_,_>>>().unwrap();
    assert_eq!(actual,expected);
    let extra:i64=c.query_row("SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' AND type!='table' AND sql IS NOT NULL",[],|r|r.get(0)).unwrap();
    assert_eq!(extra,0);
}
pub const CATALOG_SCHEMA:&str=r#"CREATE TABLE catalog_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;
CREATE TABLE catalog_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 8), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=131072), state BLOB NOT NULL CHECK(length(state)<=16384)) STRICT;
CREATE TABLE catalog_snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), rows BLOB NOT NULL CHECK(length(rows)<=1179648)) STRICT;
CREATE TABLE pending_world (singleton INTEGER PRIMARY KEY CHECK(singleton=1), sequence BLOB NOT NULL CHECK(length(sequence)=8), operation BLOB NOT NULL UNIQUE CHECK(length(operation)=16), stage INTEGER NOT NULL CHECK(stage BETWEEN 1 AND 2), record BLOB NOT NULL CHECK(length(record)<=5242880), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE operation_receipts (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), operation BLOB NOT NULL UNIQUE CHECK(length(operation)=16), result BLOB NOT NULL CHECK(length(result)<=4096), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE artifact_registry (kind INTEGER PRIMARY KEY CHECK(kind BETWEEN 1 AND 5), artifact BLOB NOT NULL UNIQUE CHECK(length(artifact)=16), path BLOB NOT NULL CHECK(length(path)<=4096), bytes BLOB NOT NULL CHECK(length(bytes)=8), digest BLOB NOT NULL CHECK(length(digest)=32), manifest BLOB NOT NULL CHECK(length(manifest)<=8192)) STRICT;
"#;
pub const CATALOG_SCHEMA_DIGEST:[u8;32]=[
0x28,0x6d,0x23,0x6b,0x6d,0xe6,0xd5,0xe8,0x9c,0x09,0x42,0x79,0x5b,0x0c,0x1f,0xd5,
0x7b,0xf2,0x6a,0x7b,0xa5,0xcb,0x61,0xa3,0x76,0x53,0xea,0xd1,0x52,0x23,0x87,0xd4];
