use super::{
    TradeChallenge, auth,
    error::{Result, TradeStoreError},
};
use crate::supplies::{
    ProofAttempt, SuppliesStore, SuppliesStoreError, auth as shared_auth, ledger, membership,
};
use nf_contract::identity::{HistoryId, UniverseId};
use nf_kernel::trade::TradeRejection;

/// A fresh signed snapshot includes membership, scope and the full closed mixed journal.
pub(super) fn read<T>(
    store: &mut SuppliesStore,
    policy: &super::policy::SelectedPolicy,
    request: TradeChallenge<'_>,
    proof: ProofAttempt,
    universe: UniverseId,
    history: HistoryId,
    lookup: impl FnOnce(&ledger::State) -> std::result::Result<T, SuppliesStoreError>,
) -> Result<T> {
    let evidence = store.auth.take(proof)?;
    if store.quarantined {
        return Err(crate::StoreError::Quarantined.into());
    }
    let tx = store.connection.transaction()?;
    let member = membership::load(
        &tx,
        nf_identity::model::Scope {
            universe: policy.supplies().universe,
            history: policy.supplies().history,
        },
    )?;
    shared_auth::verify_context(
        &evidence,
        auth::context(request, member.revision).ok_or(TradeStoreError::UnsupportedOperation)?,
        &member,
    )?;
    if universe != policy.supplies().universe || history != policy.supplies().history {
        return Err(TradeRejection::Scope.into());
    }
    if let TradeChallenge::Outbox(query) = request
        && (query.limit == 0 || query.limit > 64)
    {
        return Err(TradeRejection::Limit.into());
    }
    let state = ledger::verify(&tx, policy.supplies(), Some(policy))?;
    let outcome = lookup(&state)?;
    shared_auth::live(&evidence)?;
    tx.commit()?;
    shared_auth::live(&evidence)?;
    Ok(outcome)
}
