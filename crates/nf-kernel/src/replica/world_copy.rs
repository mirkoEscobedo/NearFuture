use crate::World;
use core::convert::Infallible;
use nf_contract::canonical::replica_budget::{BudgetResult, ReplicaDecodeScope, ScopedError};
#[path = "world_copy/reservation.rs"]
mod reservation;

/// Returns an owned data copy; this operation grants no additional authority.
pub fn clone_world_with_scope(
    world: &World,
    scope: &ReplicaDecodeScope<'_>,
) -> BudgetResult<World> {
    match scope.nested::<_, Infallible, _>(|record| {
        reservation::world(world, record)?;
        Ok(world.clone())
    }) {
        Ok(copy) => Ok(copy),
        Err(ScopedError::Budget(error)) => Err(error),
        Err(ScopedError::Semantic(never)) => match never {},
    }
}
