#![allow(dead_code)] // Shared retained fixtures are used by independent test binaries.
use super::community::Community;
use nf_contract::identity::*;
use nf_identity::model::{MembershipRepository, MembershipState};
use nf_kernel::*;
use nf_store::{PrincipalDevice, Store};
pub fn create(c: &Community, path: &std::path::Path, delta: i64, settle_now: bool) -> Store {
    create_from(&c.state, &c.server, &c.client, path, delta, settle_now)
}
pub fn create_from(
    state: &MembershipState,
    server: &nf_identity::model::PublicIdentity,
    client: &nf_identity::model::PublicIdentity,
    path: &std::path::Path,
    delta: i64,
    settle_now: bool,
) -> Store {
    let mut spec = WorldSpec::empty(state.scope.universe, state.scope.history, [3; 32], [4; 32]);
    spec.factions.push(Faction {
        id: EntityId::from_bytes([10; 16]),
        aggregate: AggregateId::from_bytes([10; 16]),
    });
    spec.markets.push(Market {
        id: EntityId::from_bytes([30; 16]),
        aggregate: AggregateId::from_bytes([30; 16]),
        faction: EntityId::from_bytes([10; 16]),
        credits: 100,
    });
    let provider = ProviderId::from_bytes([41; 16]);
    spec.providers.push(ProviderState {
        id: provider,
        aggregate: AggregateId::from_bytes([41; 16]),
        draws: 0,
        cooldown_until: WorldTick(0),
    });
    spec.registry.push(ProviderManifest {
        id: provider,
        implementation_hash: [41; 32],
        version: 1,
        state_schema: 1,
        kind: ProviderKind::Manual,
        read_domains: [Domain::Faction, Domain::Market, Domain::Provider].into(),
        write_domains: [Domain::Market, Domain::Provider].into(),
        capabilities: [Capability::AdjustMarket].into(),
        principals: [client.account].into(),
        max_events: 2,
    });
    for n in [30, 41] {
        spec.ownership.push(OwnershipRecord {
            aggregate: AggregateId::from_bytes([n; 16]),
            provider,
            generation: 1,
            activation_seq: EventSeq(0),
            ruleset_hash: [4; 32],
        });
    }
    for n in [10, 30, 41] {
        spec.revisions
            .insert(AggregateId::from_bytes([n; 16]), AggregateRevision(0));
    }
    let world = World::new(spec).unwrap();
    let intent = Intent {
        request: RequestId::from_bytes([9; 16]),
        operation: OperationId::from_bytes([9; 16]),
        job: JobId::from_bytes([9; 16]),
        actor: client.account,
        universe: state.scope.universe,
        history: state.scope.history,
        provider,
        expected: [10, 30, 41]
            .map(|n| (AggregateId::from_bytes([n; 16]), AggregateRevision(0)))
            .into(),
        command: Command::AdjustMarket {
            market: EntityId::from_bytes([30; 16]),
            delta,
        },
    };
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![intent.clone()], authority).unwrap();
    let mut store = Store::create(path, &world).unwrap();
    let founder = MembershipState::bootstrap(state.scope, server).unwrap();
    store.commit_membership(None, &founder).unwrap();
    store.commit_membership(Some(0), state).unwrap();
    store
        .prepare(
            &frontier,
            &[PrincipalDevice {
                request: intent.request,
                device: client.device,
            }],
            &[],
        )
        .unwrap();
    if settle_now {
        let Settlement::Committed { batch, .. } = settle(
            &world,
            &frontier,
            vec![],
            authority,
            MissingPolicy::Recompute,
        )
        .unwrap() else {
            panic!("deterministic setup")
        };
        store.commit(&batch).unwrap();
    }
    store
}
