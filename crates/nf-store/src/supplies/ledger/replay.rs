use super::*;
use crate::persistence::bounded_blob;
use crate::schema::read_counter;
use nf_identity::model::Scope;
use nf_kernel::supplies::origin_bytes;

pub(in crate::supplies) fn load(connection: &Connection, policy: &SuppliesPolicy) -> Result<State> {
    let scope = Scope {
        universe: policy.universe,
        history: policy.history,
    };
    let revision = crate::supplies::schema::verify(connection, scope, policy)?;
    for table in [
        "supplies_operations",
        "supplies_requests",
        "supplies_balances",
    ] {
        capacity(connection, table)?;
    }
    let mut state = State {
        revision,
        operations: BTreeMap::new(),
        requests: BTreeMap::new(),
        balances: BTreeMap::new(),
    };
    let mut stmt = connection.prepare(
        "SELECT operation,economic_digest,body,revision,decision FROM supplies_operations ORDER BY revision",
    )?;
    let mut rows = stmt.query([])?;
    let mut expected_revision = 0_u64;
    while let Some(row) = rows.next()? {
        let value = Operation::decode(&bounded_blob(row, 2, 384)?)?;
        value
            .permit(policy)
            .map_err(|_| SuppliesStoreError::Corrupt)?;
        let next = expected_revision
            .checked_add(1)
            .ok_or(SuppliesStoreError::Corrupt)?;
        if bounded_blob(row, 0, 16)? != value.id().as_bytes()
            || bounded_blob(row, 1, 32)? != value.economic()
            || read_counter(&bounded_blob(row, 3, 8)?)? != next
        {
            return Err(SuppliesStoreError::Corrupt);
        }
        let decision = Decision::decode(row.get(4)?)?;
        match decision {
            Decision::Accepted => value
                .apply(state.balances.entry(value.key()).or_default())
                .map_err(|_| SuppliesStoreError::Corrupt)?,
            Decision::InsufficientAvailable => {
                let balance = state
                    .balances
                    .get(&value.key())
                    .copied()
                    .unwrap_or_default();
                let insufficient = matches!(value, Operation::Reserve(reserve) if reserve.amount > balance.available)
                    || matches!(value, Operation::Burn(burn) if burn.amount > balance.available);
                if !insufficient {
                    return Err(SuppliesStoreError::Corrupt);
                }
            }
        }
        if state
            .operations
            .insert(
                value.id(),
                Entry {
                    operation: value,
                    revision: next,
                    decision,
                },
            )
            .is_some()
        {
            return Err(SuppliesStoreError::Corrupt);
        }
        expected_revision = next;
    }
    if expected_revision != revision {
        return Err(SuppliesStoreError::Corrupt);
    }
    let mut stmt = connection.prepare(
        "SELECT request,binding_digest,operation,body FROM supplies_requests ORDER BY request",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let value = Operation::decode(&bounded_blob(row, 3, 384)?)?;
        let original = state
            .operations
            .get(&value.id())
            .ok_or(SuppliesStoreError::Corrupt)?;
        let binding = value.binding();
        if bounded_blob(row, 0, 16)? != value.request().as_bytes()
            || bounded_blob(row, 1, 32)? != binding
            || bounded_blob(row, 2, 16)? != value.id().as_bytes()
            || value.economic() != original.operation.economic()
            || state
                .requests
                .insert(value.request(), (binding, value.id()))
                .is_some()
        {
            return Err(SuppliesStoreError::Corrupt);
        }
    }
    for entry in state.operations.values() {
        if state.requests.get(&entry.operation.request())
            != Some(&(entry.operation.binding(), entry.operation.id()))
        {
            return Err(SuppliesStoreError::Corrupt);
        }
    }
    let expected: BTreeMap<_, _> = state
        .balances
        .iter()
        .map(|((account, content, origin), value)| {
            (
                (
                    account.as_bytes().to_vec(),
                    content.to_vec(),
                    origin_bytes(*origin).to_vec(),
                ),
                balance_bytes(*value).to_vec(),
            )
        })
        .collect();
    let mut stmt = connection.prepare(
        "SELECT account,content,origin,body FROM supplies_balances ORDER BY account,content,origin",
    )?;
    let actual: BTreeMap<_, _> = stmt
        .query_map([], |r| {
            Ok((
                (
                    bounded_blob(r, 0, 16)?,
                    bounded_blob(r, 1, 32)?,
                    bounded_blob(r, 2, 36)?,
                ),
                bounded_blob(r, 3, 48)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if actual != expected {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(state)
}
