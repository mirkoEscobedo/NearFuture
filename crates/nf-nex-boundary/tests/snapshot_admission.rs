mod support;
use nf_nex_boundary::{BoundaryError, NexWorld};

#[test]
fn missing_subject_faction_cannot_be_a_usable_view() {
    let mut snapshot = support::snapshot();
    snapshot.factions.clear();
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::MissingFact("subject faction"))
    );
}

#[test]
fn copied_snapshot_preserves_provenance_float_bits_and_lifecycle() {
    let snapshot = support::snapshot();
    let world = NexWorld::admit(snapshot.clone()).unwrap();
    assert_eq!(world.snapshot(), &snapshot);
    assert_eq!(world.snapshot().concerns[0].cooldown.bits(), 0x8000_0000);
    assert_eq!(
        world.assessment().authority,
        nf_nex_boundary::Authority::ShadowOnly
    );
    assert_eq!(
        world.assessment().runtime,
        nf_nex_boundary::RuntimeCertification::Unobserved
    );
}

#[test]
fn duplicate_semantic_factions_are_not_a_coherent_view() {
    let mut snapshot = support::snapshot();
    snapshot.factions.push(snapshot.factions[0].clone());
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::Duplicate("faction"))
    );
}
