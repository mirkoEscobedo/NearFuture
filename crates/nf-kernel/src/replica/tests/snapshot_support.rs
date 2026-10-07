use super::support;
use crate::*;
use alloc::vec;
use nf_contract::identity::{AccountId, AggregateRevision, AuthorityTerm, RuntimeSession};

pub(super) fn varied() -> World {
    let mut spec = support::fixture().to_spec();
    spec.factions.reverse();
    spec.relations.reverse();
    spec.markets.reverse();
    spec.providers.reverse();
    spec.registry.reverse();
    spec.ownership.reverse();
    spec.relations[0].score = 3141;
    spec.markets[0].credits = 123456;
    for state in &mut spec.providers {
        state.draws = 19;
        state.cooldown_until = nf_contract::identity::WorldTick(9);
    }
    for manifest in &mut spec.registry {
        manifest.implementation_hash[0] = 211;
        for id in 100..112 {
            manifest.principals.insert(AccountId::from_bytes([id; 16]));
        }
        if manifest.kind == ProviderKind::Manual {
            manifest.capabilities.insert(Capability::AdjustRelation);
            manifest.read_domains.insert(Domain::Relation);
            manifest.write_domains.insert(Domain::Relation);
        }
    }
    // Reordered public input is admitted through actual World validation.
    World::new(spec).unwrap()
}

pub(super) fn larger() -> World {
    let mut spec = support::fixture().to_spec();
    for id in 50..150 {
        spec.factions.push(Faction {
            id: support::entity(id),
            aggregate: support::aggregate(id),
        });
        spec.revisions
            .insert(support::aggregate(id), AggregateRevision(0));
    }
    spec.factions.reverse();
    World::new(spec).unwrap()
}

pub(super) fn settled(rejected: bool) -> World {
    let genesis = support::fixture();
    let mut market = support::market_intent(&genesis, 2, 7);
    if rejected {
        assert!(market.expected.remove(&support::aggregate(10)).is_some());
    }
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(
        &genesis,
        vec![support::peace_intent(&genesis, 1), market],
        authority,
    )
    .unwrap();
    let Settlement::Committed { world, batch } = settle(
        &genesis,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("actual recompute must commit both outcomes")
    };
    assert_eq!(batch.outcomes.len(), 2);
    assert_eq!(world.outcomes(), batch.outcomes.as_slice());
    assert!(
        world
            .outcomes()
            .iter()
            .any(|value| value.rejection.is_none())
    );
    assert_eq!(
        world
            .outcomes()
            .iter()
            .filter(|value| value.rejection == Some(Rejection::InvalidProposal))
            .count(),
        usize::from(rejected),
    );
    *world
}
