//! Literal retired format and complete selected binding; no prefix-only acceptance.
use super::{catalog::Registry, observations::{Rows, rows_hash, readonly_public}, records::{blob,hash}, transcripts::Auth};
use rusqlite::types::Value;
use std::collections::BTreeMap;
pub const RETIRED_SCHEMA: &str = r#"
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
CREATE TABLE owner_retirement (singleton INTEGER PRIMARY KEY CHECK(singleton=1), record BLOB NOT NULL CHECK(length(record)<=8192), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
"#;
pub const RETIRED_SCHEMA_DIGEST:[u8;32]=[
0xe0,0xdf,0xbe,0x1c,0x82,0xa4,0xcc,0x23,0x7a,0x7d,0x83,0x4a,0x97,0x9a,0xc9,0x8d,
0xd5,0xc9,0x02,0x0c,0xb1,0xe7,0x81,0x69,0x50,0xce,0x2f,0x31,0xed,0x47,0xb9,0x5e];
pub fn record(auth:&Auth,registry:&BTreeMap<u32,Registry>,source_binding:&[u8],
    source_foot:&[u8],target_foot:&[u8],lineage:[u8;32])->Vec<u8>{
    assert_eq!(auth.purpose,8);assert_eq!(source_binding.len(),243);
    let mut out=b"NF-OWNER-RETIRE-1\0\x01\0".to_vec();
    out.extend_from_slice(&auth.stamp.installation);out.extend_from_slice(&auth.stamp.bytes);
    out.extend_from_slice(&auth.sequence.to_be_bytes());out.extend_from_slice(&auth.operation);
    for kind in [1,2,3]{out.extend_from_slice(&registry[&kind].id);}
    blob(&mut out,source_binding);blob(&mut out,source_foot);blob(&mut out,target_foot);
    blob(&mut out,&super::records::composition_bytes(4,super::records::PROFILE4_SCHEMA,9));
    out.extend_from_slice(&lineage);out.extend_from_slice(&registry[&4].digest);
    out.extend_from_slice(&registry[&5].digest);out.extend_from_slice(&RETIRED_SCHEMA_DIGEST);
    assert!(out.len()<=8192);out
}
pub fn rows(base:&Rows,record:&[u8])->Rows{
    let mut out=base.clone();out.insert("owner_retirement".into(),vec![vec![Value::Integer(1),
        Value::Blob(record.to_vec()),Value::Blob(hash(record).to_vec())]]);out
}
pub fn foot(source:&[u8],base:&Rows,record:&[u8])->Vec<u8>{
    let mut out=source.to_vec();let offset=b"NF-OWNER-STORAGE-1\0\x01\0".len();
    out[offset..offset+4].copy_from_slice(&5u32.to_le_bytes());
    out[offset+4..offset+36].copy_from_slice(&RETIRED_SCHEMA_DIGEST);
    let at=out.len()-32;out[at..].copy_from_slice(&rows_hash(&rows(base,record)));out
}
pub fn observe(path:&std::path::Path,base:&Rows,expected:&[u8]){
    let c=readonly_public(path);super::schemas::check(&c,0x4e46_5354,5,RETIRED_SCHEMA,RETIRED_SCHEMA_DIGEST);
    assert_eq!(super::observations::base_rows(path),*base);
    let (record,digest):(Vec<u8>,Vec<u8>)=c.query_row(
        "SELECT record,digest FROM owner_retirement WHERE singleton=1",[],
        |r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(record,expected);assert_eq!(digest,hash(expected));
    let n:i64=c.query_row("SELECT count(*) FROM owner_retirement",[],|r|r.get(0)).unwrap();assert_eq!(n,1);
}
