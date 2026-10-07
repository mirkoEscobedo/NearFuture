use nf_contract::identity::*;
use nf_kernel::miniature::*;
pub fn world() -> MiniatureWorld {
    let component = nf_world::generate(nf_world::Genesis {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
        seed: [3; 32],
        accounts: [
            AccountId::from_bytes([4; 16]),
            AccountId::from_bytes([5; 16]),
            AccountId::from_bytes([6; 16]),
        ],
    })
    .unwrap();
    MiniatureWorld::new(
        MiniatureMetadata {
            tick: WorldTick(0),
            event_sequence: EventSeq(0),
            aggregate: AggregateId::from_bytes([20; 16]),
            provider: ProviderId::from_bytes([21; 16]),
            provider_aggregate: AggregateId::from_bytes([22; 16]),
            provider_revision: AggregateRevision(0),
        },
        component,
    )
    .unwrap()
}
#[allow(dead_code)]
pub fn intent(world: &MiniatureWorld) -> MiniatureIntent {
    let g = world.component().genesis();
    let m = world.metadata();
    MiniatureIntent {
        request: RequestId::from_bytes([30; 16]),
        operation: OperationId::from_bytes([31; 16]),
        job: JobId::from_bytes([32; 16]),
        actor: g.accounts[0],
        device: DeviceId::from_bytes([33; 16]),
        universe: g.universe,
        history: g.history,
        provider: m.provider,
        expected: [
            (m.aggregate, world.component().revision()),
            (m.provider_aggregate, m.provider_revision),
        ]
        .into(),
        action: nf_world::WorldAction::CreateColony {
            site: world
                .component()
                .markets()
                .iter()
                .find(|m| m.ordinal == 1)
                .unwrap()
                .id,
            faction: world
                .component()
                .factions()
                .iter()
                .find(|f| f.ordinal == 0)
                .unwrap()
                .id,
        },
    }
}
#[allow(dead_code)]
pub fn hex(value: &str) -> Vec<u8> {
    value
        .trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
