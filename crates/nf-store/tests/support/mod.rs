#![allow(dead_code)]
use nf_contract::identity::{HistoryId, UniverseId};
use nf_kernel::{World, WorldSpec};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join(format!(
            "nf-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn db(&self) -> PathBuf {
        self.0.join("history.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .canonicalize()
            .unwrap();
        assert!(self.0.starts_with(root));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub fn world() -> World {
    World::new(WorldSpec::empty(
        UniverseId::from_bytes([1; 16]),
        HistoryId::from_bytes([2; 16]),
        [3; 32],
        [4; 32],
    ))
    .unwrap()
}

pub fn strategic_world() -> World {
    use nf_contract::identity::*;
    use nf_kernel::*;
    let mut spec = world().to_spec();
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
    spec.providers.push(ProviderState {
        id: ProviderId::from_bytes([41; 16]),
        aggregate: AggregateId::from_bytes([41; 16]),
        draws: 0,
        cooldown_until: WorldTick(0),
    });
    spec.registry.push(ProviderManifest {
        id: ProviderId::from_bytes([41; 16]),
        implementation_hash: [41; 32],
        version: 1,
        state_schema: 1,
        kind: ProviderKind::Manual,
        read_domains: [Domain::Faction, Domain::Market, Domain::Provider].into(),
        write_domains: [Domain::Market, Domain::Provider].into(),
        capabilities: [Capability::AdjustMarket].into(),
        principals: [AccountId::from_bytes([90; 16])].into(),
        max_events: 2,
    });
    for n in [30, 41] {
        spec.ownership.push(OwnershipRecord {
            aggregate: AggregateId::from_bytes([n; 16]),
            provider: ProviderId::from_bytes([41; 16]),
            generation: 1,
            activation_seq: EventSeq(0),
            ruleset_hash: [4; 32],
        });
    }
    for n in [10, 30, 41] {
        spec.revisions
            .insert(AggregateId::from_bytes([n; 16]), AggregateRevision(0));
    }
    World::new(spec).unwrap()
}
pub fn intent(world: &World, n: u8, delta: i64) -> nf_kernel::Intent {
    use nf_contract::identity::*;
    use nf_kernel::*;
    let spec = world.to_spec();
    Intent {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n; 16]),
        job: JobId::from_bytes([n; 16]),
        actor: AccountId::from_bytes([90; 16]),
        universe: spec.universe,
        history: spec.history,
        provider: ProviderId::from_bytes([41; 16]),
        expected: [10, 30, 41]
            .into_iter()
            .map(|n| {
                let id = AggregateId::from_bytes([n; 16]);
                (id, world.view().revision(id).unwrap())
            })
            .collect(),
        command: Command::AdjustMarket {
            market: EntityId::from_bytes([30; 16]),
            delta,
        },
    }
}
pub fn authority() -> nf_kernel::AuthorityContext {
    use nf_contract::identity::*;
    nf_kernel::AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    }
}
pub fn frontier(world: &World, intent: &nf_kernel::Intent) -> nf_kernel::Frontier {
    nf_kernel::admit(world, vec![intent.clone()], authority()).unwrap()
}
pub fn batch(world: &World, frontier: &nf_kernel::Frontier) -> (World, nf_kernel::CommittedBatch) {
    match nf_kernel::settle(
        world,
        frontier,
        vec![],
        authority(),
        nf_kernel::MissingPolicy::Recompute,
    )
    .unwrap()
    {
        nf_kernel::Settlement::Committed { world, batch } => (*world, batch),
        _ => panic!("Deterministic recompute must settle"),
    }
}
pub fn devices(intent: &nf_kernel::Intent) -> [nf_store::PrincipalDevice; 1] {
    [nf_store::PrincipalDevice {
        request: intent.request,
        device: device(),
    }]
}
pub fn device() -> nf_contract::identity::DeviceId {
    nf_contract::identity::DeviceId::from_bytes([91; 16])
}
