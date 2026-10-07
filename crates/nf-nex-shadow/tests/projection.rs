#[path = "../../nf-nex-boundary/tests/support/mod.rs"]
mod boundary_fixture;
use nf_contract::identity::EntityId;
use nf_nex_boundary::NexWorld;
use nf_nex_shadow::*;
#[test]
fn immutable_world_projection_preserves_state_and_refuses_unavailable_routes() {
    let snapshot = boundary_fixture::snapshot();
    let world = NexWorld::admit(snapshot.clone()).unwrap();
    let facts = war_from_world(
        &world,
        WarOperation::Update,
        Some(EntityId::from_bytes([9; 16])),
    )
    .unwrap();
    assert_eq!(facts.weariness, snapshot.weariness.adjusted);
    assert_eq!(facts.priority.existing, snapshot.concerns[0].priority);
    assert!(
        !evaluate_war(&facts, WarOperation::Update)
            .unwrap()
            .writes
            .is_empty()
    );
    assert_eq!(world.snapshot(), &snapshot);
    let new = war_from_world(&world, WarOperation::Generate, None).unwrap();
    assert!(
        !evaluate_war(&new, WarOperation::Generate)
            .unwrap()
            .generated
    );
    assert_eq!(
        selected_peace_from_world(&world),
        Err(Unavailable::MissingFact)
    );
    assert_eq!(
        war_from_world(&world, WarOperation::Update, None),
        Err(Unavailable::MissingFact)
    );
    for absent in 0..2 {
        let mut snapshot = snapshot.clone();
        if absent == 0 {
            snapshot.weariness.manager_present = false;
        } else {
            snapshot.weariness.map_entry_present = false;
        }
        assert_eq!(
            war_from_world(
                &NexWorld::admit(snapshot).unwrap(),
                WarOperation::Generate,
                None
            ),
            Err(Unavailable::MissingFact)
        );
    }
    let mut disabled = snapshot;
    disabled.concern_config.no_auto_generate = true;
    assert_eq!(
        war_from_world(
            &NexWorld::admit(disabled).unwrap(),
            WarOperation::Generate,
            None
        ),
        Err(Unavailable::Disabled)
    );
}

#[test]
fn bound_world_evaluation_refuses_mislabelled_subject_instance_and_frontier() {
    use nf_contract::identity::{BranchId, CampaignId, ProviderId};
    let snapshot = boundary_fixture::snapshot();
    let world = NexWorld::admit(snapshot.clone()).unwrap();
    let target = Some(EntityId::from_bytes([9; 16]));
    let metadata = ShadowMetadata {
        provenance: snapshot.provenance.clone(),
        subject_faction: snapshot.subject_faction.clone(),
        concern_instance: target,
        provider: ProviderId::from_bytes([1; 16]),
        campaign: CampaignId::from_bytes([2; 16]),
        branch: BranchId::from_bytes([3; 16]),
        reference_digest: [1; 32],
        corpus_digest: [2; 32],
        implementation_digest: [3; 32],
        capture_policy_digest: [4; 32],
    };
    assert!(evaluate_world_war(&metadata, &world, WarOperation::Update, target).is_ok());
    let mut changed = metadata.clone();
    changed.subject_faction = "tritachyon".into();
    assert_eq!(
        evaluate_world_war(&changed, &world, WarOperation::Update, target),
        Err(Unavailable::Stale)
    );
    changed = metadata.clone();
    changed.concern_instance = None;
    assert_eq!(
        evaluate_world_war(&changed, &world, WarOperation::Update, target),
        Err(Unavailable::Stale)
    );
    changed = metadata;
    changed.provenance.frontier.0 += 1;
    assert_eq!(
        evaluate_world_war(&changed, &world, WarOperation::Update, target),
        Err(Unavailable::Stale)
    );
    assert_eq!(world.snapshot(), &snapshot);
}

#[test]
fn world_derived_output_is_inspectable_but_unpublishable_without_full_world_commitment() {
    use nf_contract::identity::{BranchId, CampaignId, ProviderId};
    let snapshot = boundary_fixture::snapshot();
    let world = NexWorld::admit(snapshot.clone()).unwrap();
    let target = Some(EntityId::from_bytes([9; 16]));
    let metadata = ShadowMetadata {
        provenance: snapshot.provenance.clone(),
        subject_faction: snapshot.subject_faction.clone(),
        concern_instance: target,
        provider: ProviderId::from_bytes([1; 16]),
        campaign: CampaignId::from_bytes([2; 16]),
        branch: BranchId::from_bytes([3; 16]),
        reference_digest: [1; 32],
        corpus_digest: [2; 32],
        implementation_digest: [3; 32],
        capture_policy_digest: [4; 32],
    };
    let result = evaluate_world_war(&metadata, &world, WarOperation::Update, target).unwrap();
    let facts = war_from_world(&world, WarOperation::Update, target).unwrap();
    let input = ShadowInput::War {
        operation: WarOperation::Update,
        facts,
    };
    let session = ShadowSession::new(metadata.clone());
    assert_eq!(result.binding_scope(), BindingScope::WorldWithoutCommitment);
    assert_eq!(
        session.accept(&result, &metadata, &input),
        Err(Unavailable::MissingFact)
    );
}
