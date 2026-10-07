use super::error::Result;
use crate::{
    StoreError,
    schema::{APPLICATION_ID, hash},
};
use rusqlite::Connection;
/// Separate profile2 definitions; accepted profile1 schema text is never changed.
pub(crate) fn schema() -> String {
    let old = crate::schema::SCHEMA;
    let start = old
        .find("CREATE TABLE reservations ")
        .expect("accepted static schema");
    let end = start + old[start..].find(';').expect("accepted static schema") + 1;
    let mut text = old[..start].to_owned();
    text.push_str("CREATE TABLE reservations (operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16), faction_id BLOB NOT NULL CHECK(length(faction_id)=16), credits BLOB NOT NULL CHECK(length(credits)=8), supplies BLOB NOT NULL CHECK(length(supplies)=8)) STRICT;");
    text.push_str(&old[end..]);
    text = text.replace("rejection BETWEEN 0 AND 17", "rejection BETWEEN 0 AND 21");
    text.push_str("CREATE TABLE world_authority (singleton INTEGER PRIMARY KEY CHECK(singleton=1), term BLOB NOT NULL CHECK(length(term)=8), session BLOB NOT NULL CHECK(length(session)=8), account_id BLOB NOT NULL CHECK(length(account_id)=16), device_id BLOB NOT NULL CHECK(length(device_id)=16), membership_revision BLOB NOT NULL CHECK(length(membership_revision)=8)) STRICT;\n");
    text
}
pub(crate) fn digest() -> [u8; 32] {
    hash(schema().as_bytes())
}
pub(crate) fn verify(connection: &Connection) -> Result<()> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if id != APPLICATION_ID || version != 2 {
        return Err(StoreError::UnsupportedSchema.into());
    }
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::Corrupt.into());
    }
    let extra:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))",[],|row|row.get(0))?;
    if extra {
        return Err(StoreError::UnsupportedSchema.into());
    }
    let expected: std::collections::BTreeMap<_, _> = schema()
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let name = s
                .strip_prefix("CREATE TABLE ")
                .and_then(|s| s.split_whitespace().next())
                .expect("static schema table");
            (name.to_owned(), s.to_owned())
        })
        .collect();
    let mut statement=connection.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 12")?;
    let actual: std::collections::BTreeMap<String, String> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if actual != expected {
        return Err(StoreError::UnsupportedSchema.into());
    }
    Ok(())
}
