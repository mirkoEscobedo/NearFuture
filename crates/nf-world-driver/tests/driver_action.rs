use nf_contract::identity::*;
use nf_kernel::miniature::{MiniatureMetadata, MiniatureRejection, MiniatureWorld};
use nf_world::{Genesis, IndustryKind, WorldAction};
use nf_world_driver::{ActionSelection, resolve_action};
fn entity(name: &str) -> EntityId {
    let hex = include_str!("../fixtures/action-ids.tsv")
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|(key, _)| *key == name)
        .unwrap()
        .1;
    EntityId::from_bytes(
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}
#[test]
fn bounded_cli_choices_resolve_stable_typed_entities_without_unavailable_or_default_targets() {
    let component = nf_world::generate(Genesis {
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
    let world = MiniatureWorld::new(
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
    .unwrap();
    // Independent Node SHA256 entity vectors; no production resolver oracle.
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Colony {
                site: 1,
                faction: 0
            }
        ),
        Ok(WorldAction::CreateColony {
            site: entity("site1"),
            faction: entity("faction0")
        })
    );
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Build {
                market: 2,
                kind: IndustryKind::Workshop
            }
        ),
        Ok(WorldAction::BuildIndustry {
            market: entity("site2"),
            kind: IndustryKind::Workshop
        })
    );
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Relationship {
                faction: 0,
                other: 1,
                score: 123
            }
        ),
        Ok(WorldAction::SetRelationship {
            faction: entity("faction0"),
            other: entity("faction1"),
            score: 123
        })
    );
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Travel {
                faction: 0,
                destination: 1
            }
        ),
        Ok(WorldAction::Travel {
            fleet: entity("fleet0"),
            destination: entity("system1")
        })
    );
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Colony {
                site: 6,
                faction: 0
            }
        ),
        Err(MiniatureRejection::InvalidReference)
    );
    assert_eq!(
        resolve_action(
            &world,
            &ActionSelection::Relationship {
                faction: 0,
                other: 1,
                score: 10001
            }
        ),
        Err(MiniatureRejection::InvalidValue)
    );
}
