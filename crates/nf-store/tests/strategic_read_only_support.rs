mod support;
#[path = "strategic_read_only_support/mod.rs"]
mod evidence;

use nf_contract::identity::{AccountId, DeviceId, EventSeq};
use nf_identity::model::{MembershipRepository, MembershipState, Scope};
use nf_store::{KnownFrontiers, Store, StoreError, StrategicReader};

#[test]
#[ignore = "requires caller-owned NF_STRATEGIC_READER_EVIDENCE_ROOT"]
fn strategic_read_only_support_observation_replays_nonempty_history_and_protects_all_persisted_minima() {
    let case = evidence::Case::new();
    case.guard(|| {
    let world = support::strategic_world();
    let (mut public, _account_secret, _device_secret) =
        case.guard(|| nf_identity::keys::generate_identity(b"public-strategic-reader-fixture".to_vec())).unwrap();
    // Real generated public keys, with the fixture's existing public principal/device IDs.
    public.account = AccountId::from_bytes([90; 16]);
    public.device = DeviceId::from_bytes([91; 16]);
    let spec = world.to_spec();
    let member = MembershipState::bootstrap(
        Scope { universe: spec.universe, history: spec.history }, &public,
    ).unwrap();
    let member0 = nf_identity::codec::encode_state(&member).unwrap();
    let mut writer = case.guard(|| Store::create(case.original(), &world)).unwrap();
    case.guard(|| writer.commit_membership(None, &member)).unwrap();
    case.check();

    let first = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &first);
    case.guard(|| writer.prepare(&frontier, &support::devices(&first), &[])).unwrap();
    let (committed, batch) = support::batch(&world, &frontier);
    case.guard(|| writer.commit(&batch)).unwrap();
    assert_eq!(writer.world(), &committed);
    assert_eq!(committed.view().market(nf_contract::identity::EntityId::from_bytes([30; 16])).unwrap().credits, 75);
    let second = support::intent(&committed, 8, 5);
    let pending = support::frontier(&committed, &second);
    case.guard(|| writer.prepare(&pending, &support::devices(&second), &[])).unwrap();
    let old = case.guard(|| writer.known_frontiers()).unwrap();
    assert_eq!((old.event_sequence.0, old.store_revision, old.membership_revision), (1, 3, Some(0)));
    let expected_old = case.guard(|| writer.snapshot()).unwrap();
    let backup_known = case.guard(|| writer.backup_to(case.backup())).unwrap();
    assert_eq!(backup_known, old);
    case.check();

    let (advanced, second_batch) = support::batch(&committed, &pending);
    case.guard(|| writer.commit(&second_batch)).unwrap();
    let mut next_member = member.clone();
    next_member.revision = 1;
    case.guard(|| writer.commit_membership(Some(0), &next_member)).unwrap();
    let member1 = nf_identity::codec::encode_state(&next_member).unwrap();
    let newer = case.guard(|| writer.known_frontiers()).unwrap();
    assert_eq!((newer.event_sequence.0, newer.store_revision, newer.membership_revision), (2, 4, Some(1)));
    assert_eq!(advanced.view().market(nf_contract::identity::EntityId::from_bytes([30; 16])).unwrap().credits, 80);
    assert_eq!(writer.world(), &advanced);
    let expected_new = case.guard(|| writer.snapshot()).unwrap();
    case.guard(|| drop(writer));
    case.check();

    // Only fixture setup changes permissions. Every product call below is read-only.
    case.make_read_only(&case.original());
    case.make_read_only(&case.backup());
    let old_sql = evidence::SqlImage::capture(&case, &case.backup());
    old_sql.assert_fixture(old, expected_old.bytes(), &member0, true);
    let new_sql = evidence::SqlImage::capture(&case, &case.original());
    new_sql.assert_fixture(newer, expected_new.bytes(), &member1, false);
    case.retain_baselines(expected_old.bytes(), expected_new.bytes(), &member0, &member1);
    let before = case.tree();

    let reader = case.guard(|| StrategicReader::open_existing(case.backup(), old)).unwrap();
    case.check();
    assert!(case.guard(|| reader.is_read_only()).unwrap());
    assert_eq!(case.guard(|| reader.known_frontiers()).unwrap(), old);
    let snapshot = case.guard(|| reader.snapshot()).unwrap();
    assert_eq!(snapshot.revision(), 3);
    assert!(snapshot.bytes() == expected_old.bytes(), "reader must reproduce independently held canonical state");
    assert_eq!(snapshot.digest(), evidence::hash(expected_old.bytes()));
    let observation = case.guard(|| reader.support_observation()).unwrap();
    let projection = evidence::projection(&observation);
    assert_eq!(projection.as_bytes(), evidence::EXPECTED.as_bytes());
    assert_eq!(evidence::hash(projection.as_bytes()), evidence::EXPECTED_SHA256);
    assert_eq!(case.guard(|| format!("{reader:?}")), "StrategicReader { read_only: true, evidence_is_not_authority: true }");
    evidence::assert_private_fields_absent(&projection, &spec, &public, &member0, &member1, &case);
    case.guard(|| drop(snapshot));
    case.guard(|| drop(observation));
    case.guard(|| drop(reader));
    case.check();
    assert!(evidence::SqlImage::capture(&case, &case.backup()) == old_sql, "all ten backup tables preserved");
    assert!(case.tree() == before, "complete selected tree bytes and available permission fields preserved");

    for (label, stronger) in [
        ("event", KnownFrontiers { event_sequence: EventSeq(2), ..old }),
        ("store_revision", KnownFrontiers { store_revision: 4, ..old }),
        ("membership_revision", KnownFrontiers { membership_revision: Some(1), ..old }),
    ] {
        case.check();
        match case.guard(|| StrategicReader::open_existing(case.backup(), stronger)) {
            Err(error) => assert_eq!(error, StoreError::StaleBackup, "{label}"),
            Ok(_) => panic!("backup accepted stronger {label} minimum"),
        }
        case.check();
        assert!(evidence::SqlImage::capture(&case, &case.backup()) == old_sql, "{label}: backup canonical/member/mirrors unchanged");
        assert!(evidence::SqlImage::capture(&case, &case.original()) == new_sql, "{label}: newer original unchanged");
        assert!(case.tree() == before, "{label}: no tree or available permission effects");
    }

    // Reopen both histories at their independently held exact protected frontiers.
    for (path, known, expected) in [
        (case.backup(), old, &expected_old),
        (case.original(), newer, &expected_new),
    ] {
        let reader = case.guard(|| StrategicReader::open_existing(path, known)).unwrap();
        assert!(case.guard(|| reader.is_read_only()).unwrap());
        assert_eq!(case.guard(|| reader.known_frontiers()).unwrap(), known);
        let actual = case.guard(|| reader.snapshot()).unwrap();
        assert_eq!(actual.revision(), expected.revision());
        assert!(actual.bytes() == expected.bytes(), "reopened canonical state preserved");
        assert_eq!(actual.digest(), expected.digest());
        case.guard(|| drop(actual));
        case.guard(|| drop(reader));
        case.check();
    }
    assert!(evidence::SqlImage::capture(&case, &case.backup()) == old_sql);
    assert!(evidence::SqlImage::capture(&case, &case.original()) == new_sql);
    assert!(case.tree() == before, "no product output, source or available permission mutation");
    case.finish(&projection, &before);
    case.check();
    });
    case.check();
}
