use super::*;
use crate::persistence::bounded_blob;
use crate::schema::read_counter;
use nf_identity::model::Scope;
use nf_kernel::supplies::origin_bytes;

pub(in crate::supplies) fn load(
    connection: &Connection,
    policy: &SuppliesPolicy,
    trade: Option<&crate::trade::policy::SelectedPolicy>,
) -> Result<State> {
    let scope = Scope {
        universe: policy.universe,
        history: policy.history,
    };
    let revision = crate::supplies::schema::verify(connection, scope, policy, trade)?;
    for table in [
        "supplies_operations",
        "supplies_requests",
        "supplies_balances",
    ] {
        capacity(connection, table)?;
    }
    let mode = if trade.and_then(|policy| policy.accepting()).is_some() {
        LedgerMode::Accepting
    } else {
        LedgerMode::Legacy
    };
    if mode == LedgerMode::Accepting {
        capacity(connection, "trade_flows")?;
    }
    let mut state = State {
        mode,
        flows: BTreeMap::new(),
        revision,
        operations: BTreeMap::new(),
        requests: BTreeMap::new(),
        balances: BTreeMap::new(),
        offers: BTreeMap::new(),
    };
    let mut stmt = connection.prepare(
        "SELECT operation,economic_digest,body,revision,decision FROM supplies_operations ORDER BY revision",
    )?;
    let mut rows = stmt.query([])?;
    let mut expected_revision = 0_u64;
    while let Some(row) = rows.next()? {
        let value = JournalRecord::decode(
            &bounded_blob(row, 2, if trade.is_some() { 512 } else { 384 })?,
            trade,
        )?;
        value
            .permit(policy, trade)
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
        let effects = value
            .effect(decision, &state)
            .map_err(|_| SuppliesStoreError::Corrupt)?;
        match value {
            JournalRecord::ReserveOffer(offer) if decision == Decision::Accepted => {
                if state
                    .offers
                    .insert(
                        offer.terms.offer,
                        OfferHistory {
                            reservation: offer.operation,
                            finalization: None,
                        },
                    )
                    .is_some()
                {
                    return Err(SuppliesStoreError::Corrupt);
                }
            }
            JournalRecord::CancelOffer(cancel) => {
                let history = state
                    .offers
                    .get_mut(&cancel.offer)
                    .ok_or(SuppliesStoreError::Corrupt)?;
                if history.finalization.replace(cancel.operation).is_some() {
                    return Err(SuppliesStoreError::Corrupt);
                }
            }
            JournalRecord::AcceptOffer(accept) => {
                let history = state
                    .offers
                    .get_mut(&accept.offer)
                    .ok_or(SuppliesStoreError::Corrupt)?;
                if history.finalization.replace(accept.operation).is_some() {
                    return Err(SuppliesStoreError::Corrupt);
                }
            }
            JournalRecord::Supplies(_) | JournalRecord::ReserveOffer(_) => {}
        }
        for (key, balance) in effects.iter() {
            state.balances.insert(*key, *balance);
        }
        for (key, flow) in effects.flows() {
            if flow.nonzero() {
                state.flows.insert(*key, *flow);
            } else {
                state.flows.remove(key);
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
        let value = JournalRecord::decode(
            &bounded_blob(row, 3, if trade.is_some() { 512 } else { 384 })?,
            trade,
        )?;
        let original = state
            .operations
            .get(&value.id())
            .ok_or(SuppliesStoreError::Corrupt)?;
        let binding = value.binding();
        if bounded_blob(row, 0, 16)? != value.request().as_bytes()
            || bounded_blob(row, 1, 32)? != binding
            || bounded_blob(row, 2, 16)? != value.id().as_bytes()
            || !value.same_kind(original.operation)
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
    if state.mode == LedgerMode::Accepting {
        flow_cache::verify(connection, &state)?;
    }
    Ok(state)
}
