#![allow(dead_code)]
use crate::*;
use alloc::vec;
use nf_contract::identity::*;
pub fn entity(n: u8) -> EntityId {
    EntityId::from_bytes([n; 16])
}
pub fn aggregate(n: u8) -> AggregateId {
    AggregateId::from_bytes([n; 16])
}
pub fn provider(n: u8) -> ProviderId {
    ProviderId::from_bytes([n; 16])
}
pub fn fixture() -> World {
    let mut s = WorldSpec::empty(
        UniverseId::from_bytes([1; 16]),
        HistoryId::from_bytes([2; 16]),
        [3; 32],
        [4; 32],
    );
    s.factions = vec![
        Faction {
            id: entity(10),
            aggregate: aggregate(10),
        },
        Faction {
            id: entity(11),
            aggregate: aggregate(11),
        },
    ];
    s.relations = vec![Relation {
        id: entity(20),
        aggregate: aggregate(20),
        left: entity(10),
        right: entity(11),
        score: -500,
    }];
    s.markets = vec![Market {
        id: entity(30),
        aggregate: aggregate(30),
        faction: entity(10),
        credits: 100,
    }];
    for (id, kind, capability, domain) in [
        (
            40,
            ProviderKind::SyntheticPeace,
            Capability::SeekPeace,
            Domain::Relation,
        ),
        (
            41,
            ProviderKind::Manual,
            Capability::AdjustMarket,
            Domain::Market,
        ),
    ] {
        s.providers.push(ProviderState {
            id: provider(id),
            aggregate: aggregate(id),
            draws: 0,
            cooldown_until: WorldTick(0),
        });
        s.registry.push(ProviderManifest {
            id: provider(id),
            implementation_hash: [id; 32],
            version: 1,
            state_schema: 1,
            kind,
            read_domains: [Domain::Faction, domain, Domain::Provider].into(),
            write_domains: [domain, Domain::Provider].into(),
            capabilities: [capability].into(),
            principals: [AccountId::from_bytes([90; 16])].into(),
            max_events: 2,
        });
        for target in [id, if id == 40 { 20 } else { 30 }] {
            s.ownership.push(OwnershipRecord {
                aggregate: aggregate(target),
                provider: provider(id),
                generation: 1,
                activation_seq: EventSeq(0),
                ruleset_hash: [4; 32],
            });
        }
    }
    for id in [10, 11, 20, 30, 40, 41] {
        s.revisions.insert(aggregate(id), AggregateRevision(0));
    }
    World::new(s).unwrap()
}
pub fn peace_intent(world: &World, n: u8) -> Intent {
    intent(
        world,
        n,
        40,
        Command::SeekPeace {
            relation: entity(20),
        },
    )
}
pub fn market_intent(world: &World, n: u8, delta: i64) -> Intent {
    intent(
        world,
        n,
        41,
        Command::AdjustMarket {
            market: entity(30),
            delta,
        },
    )
}
fn intent(world: &World, n: u8, p: u8, command: Command) -> Intent {
    let s = world.to_spec();
    let ids = if p == 40 {
        vec![10, 11, 20, 40]
    } else {
        vec![10, 30, 41]
    };
    Intent {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n; 16]),
        job: JobId::from_bytes([n; 16]),
        actor: AccountId::from_bytes([90; 16]),
        universe: s.universe,
        history: s.history,
        provider: provider(p),
        expected: ids
            .into_iter()
            .map(|i| (aggregate(i), world.view().revision(aggregate(i)).unwrap()))
            .collect(),
        command,
    }
}
