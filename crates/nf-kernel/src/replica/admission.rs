use crate::{AuthorityContext, Frontier, Intent, Rejection, World};
use nf_contract::canonical::replica_budget::{
    BudgetResult, ReplicaDecodeScope, ScopedError, ScopedResult,
};
mod jobs;
mod limits;
mod ordering;

/// Admits borrowed intents under one cumulative logical resource scope.
/// Evaluation, settlement and frontier encoding retain their separate default APIs.
pub fn admit_with_scope(
    world: &World,
    intents: &[Intent],
    authority: AuthorityContext,
    scope: &ReplicaDecodeScope<'_>,
) -> BudgetResult<Result<Frontier, Rejection>> {
    if let Some(error) = scope.failure() {
        return Err(error);
    }
    if let Err(error) = limits::frontier(world, intents) {
        return Ok(Err(error));
    }
    match scope.nested(|frontier| admit(world, intents, authority, frontier)) {
        Ok(frontier) => Ok(Ok(frontier)),
        Err(ScopedError::Semantic(error)) => Ok(Err(error)),
        Err(ScopedError::Budget(error)) => Err(error),
    }
}

fn admit(
    world: &World,
    intents: &[Intent],
    authority: AuthorityContext,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Frontier, Rejection> {
    // The retained hash and authority have fixed logical widths32 and16.
    scope.charge_copied(48).map_err(ScopedError::Budget)?;
    ordering::deduplicate(world, intents, scope)?;
    let ordered = ordering::intents(intents, scope)?;
    let jobs = jobs::build(world, ordered, scope)?;
    let snapshot = super::clone_world_with_scope(world, scope).map_err(ScopedError::Budget)?;
    let input_hash = super::snapshot::hash(world, scope)?;
    Ok(Frontier {
        snapshot,
        input_hash,
        authority,
        jobs,
    })
}
