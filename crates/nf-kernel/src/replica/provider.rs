use crate::provider::Access;
use crate::{Capability, Command, Domain, Intent, Rejection, World};
use nf_contract::canonical::replica_budget::{
    BudgetResult, ReplicaDecodeScope, ScopedError, ScopedResult,
};
use nf_contract::identity::EntityId;
#[path = "provider/access.rs"]
mod access;

enum FactionIds {
    One([EntityId; 1]),
    Two([EntityId; 2]),
}
impl FactionIds {
    fn as_slice(&self) -> &[EntityId] {
        match self {
            Self::One(ids) => ids,
            Self::Two(ids) => ids,
        }
    }
}

pub(crate) fn validate_intent_with_scope(
    world: &World,
    intent: &Intent,
    scope: &ReplicaDecodeScope<'_>,
) -> BudgetResult<Result<Access, Rejection>> {
    if let Some(error) = scope.failure() {
        return Err(error);
    }
    match validate(world, intent, scope) {
        Ok(access) => Ok(Ok(access)),
        Err(ScopedError::Semantic(error)) => Ok(Err(error)),
        Err(ScopedError::Budget(error)) => Err(error),
    }
}
fn validate(
    world: &World,
    intent: &Intent,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Access, Rejection> {
    let spec = &world.spec;
    let manifest = spec
        .registry
        .iter()
        .find(|value| value.id == intent.provider)
        .ok_or(ScopedError::Semantic(Rejection::Unauthorized))?;
    if intent.universe != spec.universe
        || intent.history != spec.history
        || !manifest.principals.contains(&intent.actor)
    {
        return Err(ScopedError::Semantic(Rejection::Unauthorized));
    }
    let module = world
        .view()
        .provider(intent.provider)
        .ok_or(ScopedError::Semantic(Rejection::InvalidReference))?;
    let (target, domain, capability, factions) = match intent.command {
        Command::SeekPeace { relation } | Command::AdjustRelation { relation, .. } => {
            let value = world
                .view()
                .relation(relation)
                .ok_or(ScopedError::Semantic(Rejection::InvalidReference))?;
            (
                value.aggregate,
                Domain::Relation,
                if matches!(intent.command, Command::SeekPeace { .. }) {
                    Capability::SeekPeace
                } else {
                    Capability::AdjustRelation
                },
                FactionIds::Two([value.left, value.right]),
            )
        }
        Command::AdjustMarket { market, .. } => {
            let value = world
                .view()
                .market(market)
                .ok_or(ScopedError::Semantic(Rejection::InvalidReference))?;
            (
                value.aggregate,
                Domain::Market,
                Capability::AdjustMarket,
                FactionIds::One([value.faction]),
            )
        }
    };
    if !manifest.capabilities.contains(&capability)
        || ![Domain::Faction, domain, Domain::Provider]
            .iter()
            .all(|value| manifest.read_domains.contains(value))
        || ![domain, Domain::Provider]
            .iter()
            .all(|value| manifest.write_domains.contains(value))
    {
        return Err(ScopedError::Semantic(Rejection::Unauthorized));
    }
    let read_ids = access::read_ids(world, target, module.aggregate, factions.as_slice(), scope)?;
    let reads = access::reads(world, read_ids, scope)?;
    let writes = access::writes(target, module.aggregate, &reads, scope)?;
    if writes.keys().any(|id| {
        !spec
            .ownership
            .iter()
            .any(|owner| owner.aggregate == *id && owner.provider == intent.provider)
    }) {
        return Err(ScopedError::Semantic(Rejection::Unauthorized));
    }
    if intent.expected.keys().ne(reads.keys()) {
        return Err(ScopedError::Semantic(Rejection::InvalidProposal));
    }
    if intent.expected != reads {
        return Err(ScopedError::Semantic(Rejection::StaleRevision));
    }
    Ok((reads, writes))
}
