use super::support;
use nf_contract::identity::*;
use nf_kernel::*;

pub(super) fn authority() -> AuthorityContext {
    AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    }
}

fn tagged(tag: u8, number: u16) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0] = tag;
    bytes[1..3].copy_from_slice(&number.to_le_bytes());
    bytes
}

fn dense_world() -> World {
    let mut spec = support::fixture().to_spec();
    for number in 0..124 {
        let aggregate = AggregateId::from_bytes(tagged(221, number));
        spec.factions.push(Faction {
            id: EntityId::from_bytes(tagged(220, number)),
            aggregate,
        });
        spec.revisions.insert(aggregate, AggregateRevision(0));
    }
    let manual = spec
        .registry
        .iter()
        .find(|value| value.kind == ProviderKind::Manual)
        .unwrap()
        .clone();
    for number in 0..30 {
        let id = ProviderId::from_bytes(tagged(222, number));
        let aggregate = AggregateId::from_bytes(tagged(223, number));
        spec.providers.push(ProviderState {
            id,
            aggregate,
            draws: 0,
            cooldown_until: WorldTick(0),
        });
        spec.revisions.insert(aggregate, AggregateRevision(0));
        let mut manifest = manual.clone();
        manifest.id = id;
        spec.registry.push(manifest);
    }
    for manifest in &mut spec.registry {
        manifest.read_domains = [
            Domain::Faction,
            Domain::Relation,
            Domain::Market,
            Domain::Provider,
        ]
        .into();
        manifest.write_domains = [
            Domain::Faction,
            Domain::Relation,
            Domain::Market,
            Domain::Provider,
        ]
        .into();
        manifest.capabilities = match manifest.kind {
            ProviderKind::SyntheticPeace => [Capability::SeekPeace].into(),
            ProviderKind::Manual => [Capability::AdjustRelation, Capability::AdjustMarket].into(),
        };
        for number in 0..63 {
            manifest
                .principals
                .insert(AccountId::from_bytes(tagged(224, number)));
        }
        assert_eq!(manifest.principals.len(), 64);
    }
    let world = World::new(spec).unwrap();
    assert_eq!(snapshot_entries(&world), 2723);
    world
}

pub(super) fn retained_outcomes() -> World {
    let mut world = dense_world();
    for round in 0..64 {
        let prototype = support::market_intent(&world, 1, 1);
        let inputs = (0..64)
            .map(|index| {
                let mut intent = prototype.clone();
                let bytes = tagged(240, round * 64 + index + 1);
                intent.request = RequestId::from_bytes(bytes);
                intent.operation = OperationId::from_bytes(bytes);
                intent.job = JobId::from_bytes(bytes);
                intent
            })
            .collect();
        let frontier = admit(&world, inputs, authority()).unwrap();
        let Settlement::Committed { world: next, batch } = settle(
            &world,
            &frontier,
            vec![],
            authority(),
            MissingPolicy::Recompute,
        )
        .unwrap() else {
            panic!("real maximum-outcome fixture must commit")
        };
        assert_eq!(batch.outcomes.len(), 64);
        assert_eq!(
            batch
                .outcomes
                .iter()
                .filter(|value| value.rejection.is_none())
                .count(),
            1
        );
        assert_eq!(
            batch
                .outcomes
                .iter()
                .filter(|value| value.rejection == Some(Rejection::Conflict))
                .count(),
            63
        );
        world = *next;
        assert_eq!(world.outcomes().len(), usize::from(round + 1) * 64);
    }
    world
}

pub(super) fn full_expected_intents(world: &World) -> Vec<Intent> {
    let spec = world.to_spec();
    let prototype = support::market_intent(world, 1, 1);
    (0..64)
        .map(|number| {
            let mut intent = prototype.clone();
            let bytes = tagged(241, number);
            intent.request = RequestId::from_bytes(bytes);
            intent.operation = OperationId::from_bytes(bytes);
            intent.job = JobId::from_bytes(bytes);
            intent.expected = spec.revisions.clone();
            intent
        })
        .collect()
}

pub(super) fn snapshot_entries(world: &World) -> usize {
    let spec = world.to_spec();
    let outer = spec.factions.len()
        + spec.relations.len()
        + spec.markets.len()
        + spec.providers.len()
        + spec.registry.len()
        + spec.ownership.len()
        + spec.revisions.len()
        + world.outcomes().len();
    let nested = spec
        .registry
        .iter()
        .map(|manifest| {
            manifest.read_domains.len()
                + manifest.write_domains.len()
                + manifest.capabilities.len()
                + manifest.principals.len()
        })
        .sum::<usize>();
    outer + nested
}
