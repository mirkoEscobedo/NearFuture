//! A changed config is staged as data; neither artifact nor hash grants activation.
use nf_contract::identity::{AccountId, DeviceId, EntityId};
use nf_identity::{
    codec::encode_state,
    model::{MembershipRepository, MembershipState, PublicIdentity},
};
use nf_kernel::{CommittedBatch, Intent, JobResult, MissingPolicy, Settlement, World};
use nf_store::{
    Reservation, Store, StoreError,
    migration::{Composition, ProviderVersion},
    staging::{StageError, verify_config_stage},
};
use sha2::{Digest, Sha256};
mod support;

const PROFILE1_SCHEMA: [u8; 32] = [
    0x7e, 0x06, 0x49, 0x12, 0x04, 0xd7, 0xaf, 0xa6, 0x93, 0x5f, 0x76, 0xf7, 0x69, 0xcb, 0x3f, 0xbf,
    0x98, 0x1c, 0x8b, 0x7c, 0x9c, 0x65, 0x10, 0x06, 0x73, 0xce, 0x3e, 0x32, 0x01, 0x23, 0x6e, 0xbb,
];

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

// Independent complete-State oracle: encode the new World, preserve the whole protected tail.
fn expected_state(source: &[u8], world: &World) -> Vec<u8> {
    let prefix = b"NF-STORE-1\0\x01\0";
    assert!(source.len() <= 1_048_576);
    assert_eq!(&source[..prefix.len()], prefix);
    let at = prefix.len();
    let size = u32::from_le_bytes(source[at..at + 4].try_into().unwrap()) as usize;
    let tail = at.checked_add(4).unwrap().checked_add(size).unwrap();
    assert!(tail <= source.len());
    let body = nf_kernel::encode_snapshot(world).unwrap();
    let mut expected = prefix.to_vec();
    expected.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
    expected.extend_from_slice(&body);
    expected.extend_from_slice(&source[tail..]);
    expected
}

#[test]
fn config_stage_requires_drain_preserves_history_and_refuses_self_hashed_wrong_derivation() {
    let scratch = support::Scratch::new();
    let stage_path = scratch.0.join("config-stage.sqlite");
    let descriptor = ProviderVersion {
        provider: nf_contract::identity::ProviderId::from_bytes([41; 16]),
        implementation: [41; 32],
        version: 1,
        state_schema: 1,
    };
    let from = Composition::new(1, PROFILE1_SCHEMA, [2; 32], [3; 32], vec![descriptor]).unwrap();
    let to = Composition::new(1, PROFILE1_SCHEMA, [9; 32], [3; 32], vec![descriptor]).unwrap();
    let initial = support::strategic_world()
        .with_ruleset_hash(from.digest())
        .unwrap();
    let intent = support::intent(&initial, 7, -25);
    let frontier = nf_kernel::admit(&initial, vec![intent.clone()], support::authority()).unwrap();
    let mut store = Store::create(scratch.db(), &initial).unwrap();

    // Synthetic public persistence values; no keys, signatures or authority are generated.
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
    let pending = store.snapshot().unwrap();
    let pending_known = store.known_frontiers().unwrap();
    let refusal = match store.seal_drained(pending_known) {
        Err(refusal) => refusal,
        Ok(_) => panic!("pending work and its reservation cannot be sealed"),
    };
    assert_eq!(refusal.error(), StageError::NotDrained);
    let mut store = refusal.into_source();
    assert_eq!(store.snapshot().unwrap().bytes(), pending.bytes());
    assert!(store.pending().is_some());
    assert_eq!(store.reservations().count(), 1);
    assert!(!stage_path.exists());

    let (committed, batch) = evaluated(&initial, &intent);
    store.commit(&batch).unwrap();
    assert_eq!(
        committed
            .view()
            .market(EntityId::from_bytes([30; 16]))
            .unwrap()
            .credits,
        75,
    );
    assert_eq!(committed.outcomes().len(), 1);
    assert_eq!(committed.outcomes()[0].rejection, None);
    assert!(store.pending().is_none());
    assert_eq!(store.reservations().count(), 0);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.membership_revision, Some(1));
    let status = store.query(&intent, support::device()).unwrap();
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
        .config_only_transition(&from, &to, &[descriptor])
        .unwrap();
    let (source, record) = source
        .stage_config_only(transition, &stage_path)
        .unwrap_or_else(|failure| panic!("stage failed: {:?}", failure.error()));
    let expected_world = committed.with_ruleset_hash(to.digest()).unwrap();
    let expected = expected_state(snapshot.bytes(), &expected_world);
    assert_ne!(from.digest(), to.digest());
    assert_eq!(record.source_binding(), &binding);
    assert_eq!(record.world(), &expected_world);
    assert_eq!(record.snapshot().bytes(), expected.as_slice());
    assert_eq!(record.membership(), Some(&membership));
    let verified = verify_config_stage(&stage_path, &binding).unwrap();
    assert_eq!(verified.world(), &expected_world);
    assert_eq!(verified.snapshot().bytes(), expected.as_slice());
    assert_eq!(verified.membership(), Some(&membership));
    assert_eq!(binding.known_frontiers(), known);
    drop(source.into_store());
    assert_eq!(std::fs::read(scratch.db()).unwrap(), old_bytes);
    let mut old = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(old.world(), &committed);
    assert_eq!(old.query(&intent, support::device()).unwrap(), status);
    assert_eq!(old.outbox().copied().collect::<Vec<_>>(), outbox);
    assert_eq!(
        encode_state(&old.load_membership(scope).unwrap().unwrap()).unwrap(),
        encode_state(&membership).unwrap(),
    );
    drop(old);

    let stage_before = std::fs::read(&stage_path).unwrap();
    assert!(matches!(
        Store::open_existing(&stage_path, known),
        Err(StoreError::UnsupportedSchema)
    ));
    assert_eq!(std::fs::read(&stage_path).unwrap(), stage_before);
    // Another real, valid World with the same outcome IDs and counters, but debit -24.
    let alternate = support::intent(&initial, 7, -24);
    let (wrong, _) = evaluated(&initial, &alternate);
    let wrong = wrong.with_ruleset_hash(to.digest()).unwrap();
    let wrong_state = expected_state(snapshot.bytes(), &wrong);
    let digest: [u8; 32] = Sha256::digest(&wrong_state).into();
    let connection = rusqlite::Connection::open(&stage_path).unwrap();
    let id: i32 = connection
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .unwrap();
    assert_eq!(id, 0x4e46_5347);
    assert_eq!(
        connection
            .execute(
                "UPDATE migration_stage SET candidate_state=?1,candidate_digest=?2 WHERE singleton=1",
                rusqlite::params![wrong_state, digest],
            )
            .unwrap(),
        1,
    );
    drop(connection);
    assert!(matches!(
        verify_config_stage(&stage_path, &binding),
        Err(StageError::DerivationMismatch)
    ));
    assert_eq!(std::fs::read(scratch.db()).unwrap(), old_bytes);
    let mut old = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(old.world(), &committed);
    assert_eq!(old.query(&intent, support::device()).unwrap(), status);
    assert_eq!(old.outbox().copied().collect::<Vec<_>>(), outbox);
    assert_eq!(old.known_frontiers().unwrap(), known);
    assert_eq!(old.load_membership(scope).unwrap(), Some(membership));
}
