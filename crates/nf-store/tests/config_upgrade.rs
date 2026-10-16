//! NFST3 is a replayed inactive upgrade; the retained Profile1 owner remains usable.
use nf_contract::identity::{AccountId, DeviceId, EntityId, ProviderId};
use nf_identity::{
    codec::encode_state,
    model::{MembershipRepository, MembershipState, PublicIdentity},
};
use nf_kernel::{CommittedBatch, Intent, JobResult, MissingPolicy, Settlement, World};
use nf_store::{
    KnownFrontiers, Reservation, Store, StoreError,
    migration::{Composition, ProviderVersion},
    upgrade::{profile3_schema_digest, verify_config_upgrade},
};
mod config_upgrade_support;
mod support;
use config_upgrade_support::{
    PROFILE1_SCHEMA, PROFILE3_SCHEMA, base_tables, expected_action, expected_head, expected_state,
};

fn evaluated(world: &World, intent: &Intent) -> (World, CommittedBatch) {
    let authority = support::authority();
    let frontier = nf_kernel::admit(world, vec![intent.clone()], authority).unwrap();
    let proposal = nf_kernel::evaluate(&frontier, intent.job).unwrap();
    match nf_kernel::settle(
        world,
        &frontier,
        vec![JobResult {
            job: intent.job,
            result: Ok(proposal),
        }],
        authority,
        MissingPolicy::Pause,
    )
    .unwrap()
    {
        Settlement::Committed { world, batch } => (*world, batch),
        Settlement::Paused(_) => panic!("the real proposal must arrive"),
    }
}

#[test]
fn materialized_config_upgrade_replays_preserved_history_and_old_profile_refuses_it() {
    // Retain originals and any partial target on failure. No destructor deletes this evidence.
    let scratch = std::mem::ManuallyDrop::new(support::Scratch::new());
    let target = scratch.0.join("config-upgrade.sqlite");
    assert!(!target.exists());
    let descriptor = ProviderVersion {
        provider: ProviderId::from_bytes([41; 16]),
        implementation: [41; 32],
        version: 1,
        state_schema: 1,
    };
    assert_eq!(profile3_schema_digest(), PROFILE3_SCHEMA);
    let from = Composition::new(1, PROFILE1_SCHEMA, [2; 32], [3; 32], vec![descriptor]).unwrap();
    let to = Composition::new(3, PROFILE3_SCHEMA, [9; 32], [3; 32], vec![descriptor]).unwrap();
    let initial = support::strategic_world()
        .with_ruleset_hash(from.digest())
        .unwrap();
    let intent = support::intent(&initial, 7, -25);
    let frontier = support::frontier(&initial, &intent);
    let mut store = Store::create(scratch.db(), &initial).unwrap();
    let scope = store.known_frontiers().unwrap().scope;
    let public = PublicIdentity {
        account: AccountId::from_bytes([90; 16]),
        account_key: [5; 32],
        device: DeviceId::from_bytes([91; 16]),
        device_key: [6; 32],
        peer: vec![7],
    };
    let mut membership = MembershipState::bootstrap(scope, &public).unwrap();
    store.commit_membership(None, &membership).unwrap();
    membership.revision = 1;
    membership.devices.get_mut(&public.device).unwrap().revoked = true;
    membership.consumed.insert([9; 16]);
    store.commit_membership(Some(0), &membership).unwrap();
    store
        .prepare(
            &frontier,
            &support::devices(&intent),
            &[Reservation {
                operation: intent.operation,
                market: EntityId::from_bytes([30; 16]),
                amount: 25,
            }],
        )
        .unwrap();
    let (committed, batch) = evaluated(&initial, &intent);
    store.commit(&batch).unwrap();
    assert_eq!(
        committed
            .view()
            .market(EntityId::from_bytes([30; 16]))
            .unwrap()
            .credits,
        75
    );
    assert_eq!(committed.outcomes().len(), 1);
    assert_eq!(committed.outcomes()[0].rejection, None);
    assert!(store.pending().is_none());
    assert_eq!(store.reservations().count(), 0);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.store_revision, 2);
    assert_eq!(known.event_sequence.0, 1);
    assert_eq!(known.membership_revision, Some(1));
    let target_known = KnownFrontiers {
        store_revision: known.store_revision.checked_add(1).unwrap(),
        ..known
    };
    let status = store.query(&intent, support::device()).unwrap();
    assert!(status.is_some());
    let outbox = store.outbox().copied().collect::<Vec<_>>();
    assert_eq!(outbox.len(), 1);
    let snapshot = store.snapshot().unwrap();
    assert!(std::fs::metadata(scratch.db()).unwrap().len() <= 1_048_576);
    let old_bytes = std::fs::read(scratch.db()).unwrap();
    let source = store
        .seal_drained(known)
        .unwrap_or_else(|refusal| panic!("drained source refused: {:?}", refusal.error()));
    let binding = source.binding();
    let transition = source
        .config_upgrade_transition(&from, &to, &[descriptor])
        .unwrap();
    let (source, prepared) = source
        .materialize_config_upgrade(transition, &target)
        .unwrap_or_else(|failure| panic!("materialization refused: {:?}", failure.error()));
    let expected_world = committed.with_ruleset_hash(to.digest()).unwrap();
    let expected = expected_state(snapshot.bytes(), &expected_world);
    assert_ne!(from.digest(), to.digest());
    assert_eq!(prepared.source_binding(), &binding);
    assert_eq!(prepared.world(), &expected_world);
    assert_eq!(prepared.snapshot().bytes(), expected.as_slice());
    assert_eq!(prepared.snapshot().revision(), target_known.store_revision);
    assert_eq!(prepared.membership(), Some(&membership));
    assert_eq!(prepared.known_frontiers(), target_known);
    drop(prepared);
    drop(source.into_store());
    assert_eq!(std::fs::read(scratch.db()).unwrap(), old_bytes);

    // Both connections observe only caller-owned immutable artifacts after source-owner release.
    let old_connection = rusqlite::Connection::open_with_flags(
        scratch.db(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let original_tables = base_tables(&old_connection);
    let old_head: Vec<u8> = old_connection
        .query_row(
            "SELECT head FROM history_meta WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let old_head: [u8; 32] = old_head.try_into().unwrap();
    drop(old_connection);
    let target_bytes = std::fs::read(&target).unwrap();
    assert!(target_bytes.len() <= 1_048_576);
    let verified = verify_config_upgrade(&target, &binding, to.digest(), target_known).unwrap();
    assert_eq!(verified.source_binding(), &binding);
    assert_eq!(verified.world(), &expected_world);
    assert_eq!(verified.snapshot().bytes(), expected.as_slice());
    assert_eq!(verified.snapshot().revision(), target_known.store_revision);
    assert_eq!(verified.membership(), Some(&membership));
    assert_eq!(verified.known_frontiers(), target_known);
    drop(verified);

    let connection =
        rusqlite::Connection::open_with_flags(&target, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    assert_eq!(
        connection
            .pragma_query_value::<i32, _>(None, "application_id", |r| r.get(0))
            .unwrap(),
        0x4e46_5354
    );
    assert_eq!(
        connection
            .pragma_query_value::<i32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
    assert_eq!(base_tables(&connection), original_tables);
    let member_digest: [u8; 32] = config_upgrade_support::hash(&encode_state(&membership).unwrap());
    let action = expected_action(known, old_head, snapshot.digest(), member_digest);
    assert!(action.len() <= 4096);
    let digest = config_upgrade_support::hash(&expected);
    let head = expected_head(
        old_head,
        target_known.store_revision,
        &action,
        snapshot.digest(),
        digest,
    );
    let row = config_upgrade_support::upgrade_row(&connection);
    assert_eq!(row.revision, target_known.store_revision.to_be_bytes());
    assert_eq!(row.kind, 4);
    assert_eq!(row.previous, snapshot.digest());
    assert_eq!(row.digest, digest);
    assert_eq!(row.head, head);
    assert_eq!(row.action, action);
    assert_eq!(row.state, expected);
    let meta = config_upgrade_support::upgrade_meta(&connection);
    assert_eq!(meta.revision, row.revision);
    assert_eq!(meta.event, known.event_sequence.0.to_be_bytes());
    assert_eq!(meta.state, row.state);
    assert_eq!(meta.digest, row.digest);
    assert_eq!(meta.head, row.head);
    assert_eq!(meta.schema, PROFILE3_SCHEMA);
    drop(connection);
    assert_eq!(std::fs::read(&target).unwrap(), target_bytes);
    assert!(matches!(
        Store::open_existing(&target, target_known),
        Err(StoreError::UnsupportedSchema)
    ));
    assert_eq!(std::fs::read(&target).unwrap(), target_bytes);
    let mut original = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(original.world(), &committed);
    assert_eq!(original.snapshot().unwrap().bytes(), snapshot.bytes());
    assert_eq!(original.query(&intent, support::device()).unwrap(), status);
    assert_eq!(original.outbox().copied().collect::<Vec<_>>(), outbox);
    assert_eq!(original.load_membership(scope).unwrap(), Some(membership));
    assert_eq!(original.known_frontiers().unwrap(), known);
    drop(original);
    assert_eq!(std::fs::read(scratch.db()).unwrap(), old_bytes);
}
