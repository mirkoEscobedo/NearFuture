use nf_contract::identity::*;
use nf_kernel::*;
#[test]
fn invalid_relation_references_never_create_a_world() {
    let mut spec = WorldSpec::empty(
        UniverseId::from_bytes([1; 16]),
        HistoryId::from_bytes([2; 16]),
        [3; 32],
        [4; 32],
    );
    spec.relations.push(Relation {
        id: EntityId::from_bytes([5; 16]),
        aggregate: AggregateId::from_bytes([6; 16]),
        left: EntityId::from_bytes([7; 16]),
        right: EntityId::from_bytes([8; 16]),
        score: 0,
    });
    assert_eq!(World::new(spec), Err(Rejection::InvalidReference));
}
