//! Exact nonzero transfer cache, derived solely from accepted sequential journal effects.
use super::*;
use crate::persistence::bounded_blob;
use nf_kernel::supplies::origin_bytes;
use rusqlite::params;

pub(super) const SCHEMA: &str = "CREATE TABLE trade_flows (account BLOB NOT NULL CHECK(length(account)=16), content BLOB NOT NULL CHECK(length(content)=32), origin BLOB NOT NULL CHECK(length(origin)=36), body BLOB NOT NULL CHECK(length(body)=16), PRIMARY KEY(account,content,origin)) STRICT;\n";

pub(super) fn write(connection: &Connection, effects: &Effects) -> Result<()> {
    for (key, flow) in effects.flows() {
        if flow.nonzero() {
            connection.execute("INSERT INTO trade_flows VALUES(?1,?2,?3,?4) ON CONFLICT(account,content,origin) DO UPDATE SET body=excluded.body",
                params![key.0.as_bytes(), key.1, origin_bytes(key.2), flow.bytes()])?;
        } else {
            connection.execute(
                "DELETE FROM trade_flows WHERE account=?1 AND content=?2 AND origin=?3",
                params![key.0.as_bytes(), key.1, origin_bytes(key.2)],
            )?;
        }
    }
    Ok(())
}
pub(super) fn verify(connection: &Connection, state: &State) -> Result<()> {
    capacity(connection, "trade_flows")?;
    flow::validate(&state.balances, &state.flows)?;
    let expected: BTreeMap<_, _> = state
        .flows
        .iter()
        .map(|((account, content, origin), value)| {
            (
                (
                    account.as_bytes().to_vec(),
                    content.to_vec(),
                    origin_bytes(*origin).to_vec(),
                ),
                value.bytes().to_vec(),
            )
        })
        .collect();
    let mut stmt = connection.prepare(
        "SELECT account,content,origin,body FROM trade_flows ORDER BY account,content,origin",
    )?;
    let actual: BTreeMap<_, _> = stmt
        .query_map([], |row| {
            Ok((
                (
                    bounded_blob(row, 0, 16)?,
                    bounded_blob(row, 1, 32)?,
                    bounded_blob(row, 2, 36)?,
                ),
                bounded_blob(row, 3, 16)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if actual != expected {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(())
}
