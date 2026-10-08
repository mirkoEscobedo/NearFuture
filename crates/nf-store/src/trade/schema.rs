use crate::{
    schema::{counter, read_counter},
    supplies::SuppliesStoreError,
};
use nf_kernel::supplies::SuppliesRejection;
use rusqlite::{Connection, params};
type Result<T> = std::result::Result<T, SuppliesStoreError>;
pub(crate) const SCHEMA: &str = "CREATE TABLE trade_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), policy_body BLOB NOT NULL CHECK(length(policy_body)<=16384), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), clock_tick BLOB NOT NULL CHECK(length(clock_tick)=8), clock_revision BLOB NOT NULL CHECK(length(clock_revision)=8)) STRICT;\n";
pub(crate) fn initialize(
    connection: &Connection,
    policy: &super::policy::SelectedPolicy,
) -> Result<()> {
    connection.execute(
        "INSERT INTO trade_meta VALUES(1,?1,?2,?3,?4)",
        params![
            policy.bytes().map_err(|_| SuppliesRejection::Policy)?,
            policy.digest().map_err(|_| SuppliesRejection::Policy)?,
            counter(0),
            counter(0)
        ],
    )?;
    Ok(())
}
pub(crate) fn verify(
    connection: &Connection,
    policy: &super::policy::SelectedPolicy,
) -> Result<(u64, u64)> {
    let (body, digest, tick, revision): (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT policy_body,policy_digest,clock_tick,clock_revision FROM trade_meta WHERE singleton=1", [], |r| Ok((
            crate::persistence::bounded_blob(r,0,16384)?, crate::persistence::bounded_blob(r,1,32)?,
            crate::persistence::bounded_blob(r,2,8)?, crate::persistence::bounded_blob(r,3,8)?)))?;
    if body != policy.bytes().map_err(|_| SuppliesRejection::Policy)?
        || digest != policy.digest().map_err(|_| SuppliesRejection::Policy)?
        || read_counter(&tick)? != 0
        || read_counter(&revision)? != 0
    {
        return Err(SuppliesStoreError::Corrupt);
    }
    // This first seam has only durable genesis; clock advancement is not implemented.
    Ok((read_counter(&tick)?, read_counter(&revision)?))
}
