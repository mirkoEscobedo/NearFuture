//! First genuine private-owner lifecycle; retained executable operation is NOT QUALIFIED.
use nf_contract::identity::{AccountId, DeviceId, EntityId};
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipRepository, MembershipState, PublicIdentity},
};
use nf_kernel::{JobResult, MissingPolicy, Rejection};
use nf_store::{
    KnownFrontiers, PrincipalDevice, Reservation, Store, StoreError,
    managed::{
        AdoptionInput, BinaryEvidence, LocalCoordinator, ManagedError, PendingPolicy, PrepareInput,
        QuiescePolicy, SettlementInput,
    },
};
mod managed_config_upgrade_support;
mod support;
use managed_config_upgrade_support as e;
#[test]
#[ignore = "requires caller-created separately custodied evidence and binary roots"]
fn managed_upgrade_restarts_fences_old_work_and_commits_new_history() {
    let case = e::Case::new();
    let mut audit = e::Audit::new(case.start(), case.cutoff());
    let account_key = SecretSeed::generate().unwrap();
    let device_key = SecretSeed::generate().unwrap();
    let public = PublicIdentity {
        account: AccountId::from_bytes([90; 16]),
        account_key: account_key.public_key(),
        device: DeviceId::from_bytes([92; 16]),
        device_key: device_key.public_key(),
        peer: b"managed-upgrade-first-owned-peer".to_vec(),
    };
    let descriptor = e::descriptor();
    let from = e::composition(1, e::PROFILE1_SCHEMA, 2);
    let to = e::composition(4, e::PROFILE4_SCHEMA, 9);
    let mut spec = support::strategic_world().to_spec();
    spec.universe =
        nf_contract::identity::UniverseId::from_bytes(nf_identity::keys::random_id().unwrap());
    spec.history =
        nf_contract::identity::HistoryId::from_bytes(nf_identity::keys::random_id().unwrap());
    let initial = nf_kernel::World::new(spec)
        .unwrap()
        .with_ruleset_hash(from.digest())
        .unwrap();
    let first = support::intent(&initial, 7, -25);
    // Existing pure fixture authority creates legacy history. It is no managed grant.
    let historical = support::authority();
    let frontier = nf_kernel::admit(&initial, vec![first.clone()], historical).unwrap();
    let first_result = JobResult {
        job: first.job,
        result: Ok(nf_kernel::evaluate(&frontier, first.job).unwrap()),
    };
    let (committed, first_batch) = e::settled(&initial, &frontier, vec![first_result], historical);
    let mut raw = Store::create(case.source(), &initial).unwrap();
    let scope = raw.known_frontiers().unwrap().scope;
    let member = MembershipState::bootstrap(scope, &public).unwrap();
    assert_eq!(member.owner, public.account);
    assert!(!member.devices[&public.device].revoked);
    assert!(
        member.accounts[&public.account]
            .roles
            .contains(nf_identity::model::Roles::AUTHORITY_CANDIDATE)
    );
    raw.commit_membership(None, &member).unwrap();
    raw.prepare(
        &frontier,
        &[PrincipalDevice {
            request: first.request,
            device: public.device,
        }],
        &[Reservation {
            operation: first.operation,
            market: EntityId::from_bytes([30; 16]),
            amount: 25,
        }],
    )
    .unwrap();
    raw.commit(&first_batch).unwrap();
    assert!(raw.page_count().unwrap() <= 57_344);
    let source_known = raw.known_frontiers().unwrap();
    assert_eq!(
        (
            source_known.store_revision,
            source_known.event_sequence.0,
            source_known.membership_revision
        ),
        (2, 1, Some(0))
    );
    let source_core = e::completed_core(&committed, &[(&first, public.device, &first_batch)]);
    assert_eq!(raw.snapshot().unwrap().bytes(), source_core);
    assert_eq!(raw.outbox().count(), 1);
    drop(raw);
    case.check();
    let original = e::bounded_read(&case.source());
    let base = e::base_rows(&case.source());
    let source_head = e::source_head(&case.source());
    let source_foot = e::source_foot(&base, &member);
    let source_envelope = e::envelope(&source_core, &member, None);
    let old_pin = case.binary_pin(case.old_binary());
    let new_pin = case.binary_pin(case.new_binary());
    let binary = BinaryEvidence::library_fixture(
        old_pin,
        new_pin,
        &from,
        &to,
        "REAL_TEST_EXECUTABLE_BYTES; OPERATIONAL_OLD_NEW_NODE_COMPATIBILITY_NOT_QUALIFIED",
    );
    let source = Store::open_existing(case.source(), source_known).unwrap();
    let input = AdoptionInput {
        source_path: case.source(),
        minimum: source_known,
        composition: &from,
        supported: &[descriptor],
        binary_evidence: binary,
        artifact_provenance: e::DATA_PROVENANCE,
    };
    let mut draft = LocalCoordinator::begin_adoption(source, case.catalog(), input)
        .unwrap_or_else(|failure| panic!("adoption refused: {:?}", failure.error()));
    let challenge = draft
        .issue_adoption_challenge(&public, case.cutoff())
        .unwrap();
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    let catalog_path = case.catalog().to_str().unwrap().to_string();
    let source_path = case.source().to_str().unwrap().to_string();
    let old_path = case.old_binary().to_str().unwrap().to_string();
    let new_path = case.new_binary().to_str().unwrap().to_string();
    let candidate = e::prospective(
        challenge.transcript(),
        challenge.admission_input_bytes(),
        9,
        e::ProspectiveExpected {
            foot: &source_foot,
            catalog: None,
            public: &public,
            member: &member,
            paths: &[&catalog_path, &source_path, &old_path, &new_path],
            composition: &e::composition_bytes(1, e::PROFILE1_SCHEMA, 2),
        },
        &mut audit,
    );
    e::assert_epoch(&candidate, 1, 1, 1, 1, from.digest());
    let mut registry = std::collections::BTreeMap::new();
    e::admission_delta(challenge.admission_input_bytes(), &mut registry);
    e::assert_adoption_operation(challenge.admission_input_bytes(), &registry);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    e::authorize(&member, &proof);
    let mut expected = e::Expected::new(
        candidate.installation,
        &source_foot,
        &registry,
        case.start(),
        case.cutoff(),
    );
    expected.signed(challenge.transcript(), &proof);
    let mut owner = draft
        .finish_adoption(proof)
        .unwrap_or_else(|failure| panic!("adoption outcome: {:?}", failure.error()));
    assert_eq!(owner.owner_stamp().canonical_bytes(), candidate.bytes);
    assert_eq!(owner.storage_foot().canonical_bytes(), source_foot);
    let old_stamp = owner.owner_stamp().clone();
    let old_context = owner.authority();
    let old_intent = support::intent(owner.world(), 8, -24);
    let old_frontier =
        nf_kernel::admit(owner.world(), vec![old_intent.clone()], old_context).unwrap();
    let old_result = JobResult {
        job: old_intent.job,
        result: Ok(nf_kernel::evaluate(&old_frontier, old_intent.job).unwrap()),
    };
    let cut = case.cutoff();
    let challenge = owner
        .issue_quiesce_challenge(QuiescePolicy::RequireAlreadyDrained, &public, cut)
        .unwrap();
    let unsigned = e::quiesce_payload(&source_foot, cut);
    let _auth = e::owned(
        challenge.transcript(),
        11,
        e::ExpectedPre {
            stamp: &candidate.bytes,
            revision: 2,
            digest: e::hash(&source_envelope),
            head: source_head,
        },
        &unsigned,
        &mut audit,
    );
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    assert_eq!(challenge.unsigned_payload(), unsigned);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed_owned(challenge.transcript(), &proof, &unsigned);
    let drained = owner
        .quiesce_and_seal(QuiescePolicy::RequireAlreadyDrained, source_known, proof)
        .unwrap_or_else(|failure| panic!("quiesce refused: {:?}", failure.error()));
    // Materialization creates data only; real manifests exist before registration signing.
    let (mut drained, prepared) = drained
        .materialize_active_upgrade(
            &from,
            &to,
            &[descriptor],
            case.target(),
            case.retained_source(),
        )
        .unwrap_or_else(|failure| panic!("stage refused: {:?}", failure.error()));
    let target_world = committed.with_ruleset_hash(to.digest()).unwrap();
    let target_core = e::completed_core(&target_world, &[(&first, public.device, &first_batch)]);
    let target_known = KnownFrontiers {
        store_revision: 3,
        ..source_known
    };
    let target_envelope = e::envelope(&target_core, &member, None);
    let source_binding = e::source_binding(
        source_known,
        source_head,
        e::hash(&source_core),
        e::hash(&nf_identity::codec::encode_state(&member).unwrap()),
        e::rows_hash(&base),
    );
    let action = e::upgrade_action(
        source_known,
        source_head,
        e::hash(&source_core),
        e::hash(&nf_identity::codec::encode_state(&member).unwrap()),
        e::rows_hash(&base),
    );
    let lineage = e::lineage(
        &action,
        3,
        e::hash(&source_envelope),
        e::hash(&target_envelope),
    );
    let initial_head = e::journal_head(
        source_head,
        3,
        4,
        &action,
        e::hash(&source_envelope),
        e::hash(&target_envelope),
        e::hash(&lineage),
    );
    let mut oracle = e::WorldOracle::initial(
        &base,
        target_known,
        &target_envelope,
        &action,
        &lineage,
        e::hash(&source_envelope),
        initial_head,
    );
    let initial_foot = oracle.foot();
    oracle.observe(&case.target());
    assert_eq!(prepared.world(), &target_world);
    assert_eq!(prepared.known_frontiers(), target_known);
    assert_eq!(prepared.core_bytes(), target_core);
    assert_eq!(prepared.envelope_bytes(), target_envelope);
    assert_eq!(prepared.storage_foot().canonical_bytes(), initial_foot);
    assert_eq!(e::bounded_read(&case.source()), original);
    assert_eq!(e::bounded_read(&case.retained_source()), original);
    case.check();
    let challenge = drained
        .issue_registration_challenge(&prepared, case.retained_source(), &public, case.cutoff())
        .unwrap();
    e::registration_delta(challenge.registration_delta_bytes(), &mut registry);
    let unsigned = e::registration_payload(&registry, challenge.registration_delta_bytes());
    let _auth = e::owned(
        challenge.transcript(),
        12,
        e::ExpectedPre {
            stamp: &candidate.bytes,
            revision: 2,
            digest: e::hash(&source_envelope),
            head: source_head,
        },
        &unsigned,
        &mut audit,
    );
    assert_eq!(challenge.unsigned_payload(), unsigned);
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed_owned(challenge.transcript(), &proof, &unsigned);
    let (mut drained, prepared) = drained
        .register_inactive_artifacts(prepared, proof)
        .unwrap();
    expected.registry = registry;
    expected.pins = e::artifact_pins(&case, &original, &from, &to);
    expected.initial = initial_foot.clone();
    e::verify_manifests(&expected, &member);
    let challenge = drained
        .issue_retirement_challenge(&prepared, case.retained_source(), &public, case.cutoff())
        .unwrap();
    let retire_auth = e::owned(
        challenge.transcript(),
        8,
        e::ExpectedPre {
            stamp: &candidate.bytes,
            revision: 2,
            digest: e::hash(&source_envelope),
            head: source_head,
        },
        challenge.unsigned_payload(),
        &mut audit,
    );
    assert_eq!(retire_auth.sequence, 1);
    let retirement = e::retirement_record(
        &retire_auth,
        &expected.registry,
        &source_binding,
        &source_foot,
        &initial_foot,
        e::hash(&lineage),
    );
    assert_eq!(challenge.unsigned_payload(), retirement);
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    expected.retirement = retirement.clone();
    expected.retired = e::retired_foot(&source_foot, &base, &retirement);
    expected.operations.insert(
        1,
        e::Operation {
            effect: e::physical_effect(
                8,
                1,
                5,
                &base,
                &e::retired_rows(&base, &retirement),
                &source_foot,
                &expected.retired,
            ),
            auth: retire_auth.clone(),
            action: vec![],
            pre: source_foot.clone(),
            post: expected.retired.clone(),
            pre_state: source_envelope.clone(),
            post_state: source_envelope.clone(),
        },
    );
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed(challenge.transcript(), &proof);
    expected.operations.get_mut(&1).unwrap().action =
        e::authenticated_action(&retire_auth, &retirement, &proof);
    let mut fenced = drained
        .retire_source(prepared, case.retained_source(), proof)
        .unwrap_or_else(|outcome| panic!("retirement needs resolution: {:?}", outcome.error()));
    e::observe_retired(&case.source(), &base, &retirement);
    assert_eq!(e::bounded_read(&case.retained_source()), original);
    oracle.observe(&case.target());
    // Library recovery reads a separate retained image; no mutation or managed writer grant.
    let legacy_reader = Store::open_existing(case.retained_source(), source_known).unwrap();
    assert_eq!(legacy_reader.world(), &committed);
    assert_eq!(legacy_reader.snapshot().unwrap().bytes(), source_core);
    assert_eq!(legacy_reader.known_frontiers().unwrap(), source_known);
    assert_eq!(legacy_reader.outbox().count(), 1);
    drop(legacy_reader);
    assert_eq!(e::bounded_read(&case.retained_source()), original);
    let source_fenced = fenced.catalog_frontier();
    let challenge = fenced
        .issue_selection_challenge(
            case.target(),
            to.digest(),
            target_known,
            &public,
            case.cutoff(),
        )
        .unwrap();
    let next = e::prospective(
        challenge.transcript(),
        challenge.admission_input_bytes(),
        13,
        e::ProspectiveExpected {
            foot: &expected.retired,
            catalog: Some(e::catalog_tuple(&source_fenced)),
            public: &public,
            member: &member,
            paths: &[&catalog_path],
            composition: &e::composition_bytes(4, e::PROFILE4_SCHEMA, 9),
        },
        &mut audit,
    );
    e::assert_epoch(&next, 2, 2, 2, 4, to.digest());
    e::assert_selection_operation(challenge.admission_input_bytes(), &expected, &next);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed(challenge.transcript(), &proof);
    let selected = fenced.select_target(proof).unwrap();
    let installation = selected.installation();
    assert_eq!(installation, candidate.installation);
    let minimum = selected.catalog_frontier();
    drop(selected);
    case.check();
    e::replay_catalog(
        &case.catalog(),
        &member,
        &expected,
        &initial_foot,
        None,
        e::catalog_tuple(&minimum),
    );
    for (path, known) in [(case.source(), source_known), (case.target(), target_known)] {
        let before = e::bounded_read(&path);
        assert!(matches!(
            Store::open_existing(&path, known),
            Err(StoreError::UnsupportedSchema)
        ));
        assert_eq!(e::bounded_read(&path), before);
    }
    let mut unclaimed =
        LocalCoordinator::open_existing(case.catalog(), installation, minimum).unwrap();
    let challenge = unclaimed
        .issue_owner_challenge(&public, &initial_foot, case.cutoff())
        .unwrap();
    let fresh = e::prospective(
        challenge.transcript(),
        challenge.admission_input_bytes(),
        10,
        e::ProspectiveExpected {
            foot: &initial_foot,
            catalog: Some(e::catalog_tuple(&minimum)),
            public: &public,
            member: &member,
            paths: &[&catalog_path],
            composition: &[],
        },
        &mut audit,
    );
    e::assert_epoch(&fresh, 2, 2, 2, 4, to.digest());
    e::assert_claim_operation(challenge.admission_input_bytes(), &fresh);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed(challenge.transcript(), &proof);
    let mut active = unclaimed.claim(proof).unwrap();
    assert_eq!(active.owner_stamp().canonical_bytes(), fresh.bytes);
    assert_eq!(active.world(), &target_world);
    assert_ne!(active.authority(), old_context);
    let baseline_core = active.core_bytes().to_vec();
    let baseline_envelope = active.envelope_bytes().to_vec();
    let baseline_known = active.known_frontiers();
    let baseline_stamp = active.owner_stamp().canonical_bytes().to_vec();
    let baseline_foot = active.storage_foot().canonical_bytes().to_vec();
    let baseline_catalog = active.catalog_frontier();
    // Quiescent byte reads under the same supported exclusive owner, no transaction active.
    let world_bytes = e::bounded_read(&case.target());
    let catalog_bytes = e::bounded_read(&case.catalog());
    assert!(matches!(
        active.admit_worker_reply(&old_stamp, &old_result),
        Err(ManagedError::StaleSession)
    ));
    assert_eq!(active.world(), &target_world);
    assert_eq!(active.core_bytes(), baseline_core);
    assert_eq!(active.envelope_bytes(), baseline_envelope);
    assert_eq!(active.known_frontiers(), baseline_known);
    assert_eq!(active.owner_stamp().canonical_bytes(), baseline_stamp);
    assert_eq!(active.storage_foot().canonical_bytes(), baseline_foot);
    assert_eq!(
        e::catalog_tuple(&active.catalog_frontier()),
        e::catalog_tuple(&baseline_catalog)
    );
    // Drop releases connections only; no close command or claim enters the refusal baseline.
    drop(active);
    assert_eq!(e::bounded_read(&case.target()), world_bytes);
    assert_eq!(e::bounded_read(&case.catalog()), catalog_bytes);
    oracle.observe(&case.target());
    e::replay_catalog(
        &case.catalog(),
        &member,
        &expected,
        &initial_foot,
        Some(&fresh),
        e::catalog_tuple(&baseline_catalog),
    );
    assert_eq!(
        nf_kernel::settle(
            &target_world,
            &old_frontier,
            vec![old_result],
            old_context,
            MissingPolicy::Pause
        ),
        Err(Rejection::StateMismatch)
    );
    let mut unclaimed =
        LocalCoordinator::open_existing(case.catalog(), installation, baseline_catalog).unwrap();
    let challenge = unclaimed
        .issue_owner_challenge(&public, &initial_foot, case.cutoff())
        .unwrap();
    let current = e::prospective(
        challenge.transcript(),
        challenge.admission_input_bytes(),
        10,
        e::ProspectiveExpected {
            foot: &initial_foot,
            catalog: Some(e::catalog_tuple(&baseline_catalog)),
            public: &public,
            member: &member,
            paths: &[&catalog_path],
            composition: &[],
        },
        &mut audit,
    );
    e::assert_epoch(&current, 2, 3, 3, 4, to.digest());
    e::assert_claim_operation(challenge.admission_input_bytes(), &current);
    assert_ne!(current.incarnation, fresh.incarnation);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed(challenge.transcript(), &proof);
    let mut active = unclaimed.claim(proof).unwrap();
    assert_eq!(active.owner_stamp().canonical_bytes(), current.bytes);
    let second = support::intent(active.world(), 9, -24);
    let context = active.authority();
    let frontier = nf_kernel::admit(active.world(), vec![second.clone()], context).unwrap();
    let pc = active
        .issue_principal_challenge(&frontier, second.request, &public, case.cutoff())
        .unwrap();
    let (pseq, pid, pnonce, pissued, pexpires) = e::principal(
        pc.transcript(),
        e::ExpectedPre {
            stamp: &current.bytes,
            revision: 3,
            digest: e::hash(&target_envelope),
            head: initial_head,
        },
        &frontier,
        second.request,
        public.device,
        &mut audit,
    );
    assert_eq!(pc.digest(), e::hash(pc.transcript()));
    let principal_proof = e::sign(&public, &device_key, scope, 0, pc.digest());
    let principal = pc.bind_proof(principal_proof.clone());
    let devices = [PrincipalDevice {
        request: second.request,
        device: public.device,
    }];
    let reservations = [Reservation {
        operation: second.operation,
        market: EntityId::from_bytes([30; 16]),
        amount: 24,
    }];
    let principals = [principal];
    let admitted = e::now_ms();
    let cut = case.cutoff();
    let prepare = PrepareInput {
        frontier: &frontier,
        devices: &devices,
        reservations: &reservations,
        principals: &principals,
        policy: PendingPolicy::Drain,
        admitted_at_ms: admitted,
        cutoff_ms: cut,
    };
    let payload = e::prepare_payload(
        &frontier,
        public.device,
        &reservations[0],
        &principal_proof,
        e::PrepareTimes {
            nonce: pnonce,
            issued: pissued,
            expires: pexpires,
            admitted,
            cutoff: cut,
        },
    );
    let challenge = active
        .issue_prepare_challenge(&prepare, &public, case.cutoff())
        .unwrap();
    let auth = e::owned(
        challenge.transcript(),
        1,
        e::ExpectedPre {
            stamp: &current.bytes,
            revision: 3,
            digest: e::hash(&target_envelope),
            head: initial_head,
        },
        &payload,
        &mut audit,
    );
    assert_eq!((auth.sequence, auth.operation), (pseq, pid));
    assert_eq!(auth.sequence, 2);
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    assert_eq!(challenge.unsigned_payload(), payload);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    let prepare_action = e::authenticated_action(&auth, &payload, &proof);
    let prepared_core = e::prepared_core(
        &target_world,
        &first,
        public.device,
        &first_batch,
        &second,
        &frontier,
        &reservations[0],
    );
    let prepared_envelope = e::envelope(&prepared_core, &member, Some((admitted, cut)));
    let pre_rows = oracle.rows.clone();
    oracle.append(
        KnownFrontiers {
            store_revision: 4,
            ..target_known
        },
        1,
        &prepare_action,
        &prepared_envelope,
        None,
    );
    expected.operations.insert(
        2,
        e::Operation {
            effect: e::physical_effect(
                1,
                4,
                4,
                &pre_rows,
                &oracle.rows,
                &initial_foot,
                &oracle.foot(),
            ),
            auth,
            action: prepare_action,
            pre: initial_foot.clone(),
            post: oracle.foot(),
            pre_state: target_envelope.clone(),
            post_state: prepared_envelope.clone(),
        },
    );
    active.prepare(prepare, proof).unwrap();
    assert_eq!(active.core_bytes(), prepared_core);
    assert_eq!(active.envelope_bytes(), prepared_envelope);
    assert_eq!(active.storage_foot().canonical_bytes(), oracle.foot());
    let result = JobResult {
        job: second.job,
        result: Ok(nf_kernel::evaluate(&frontier, second.job).unwrap()),
    };
    let (final_world, batch) = e::settled(&target_world, &frontier, vec![result.clone()], context);
    let settlement = SettlementInput {
        results: vec![result.clone()],
        missing_policy: MissingPolicy::Pause,
    };
    let payload = e::settlement_payload(&[result], MissingPolicy::Pause);
    let challenge = active
        .issue_settlement_challenge(&settlement, &public, case.cutoff())
        .unwrap();
    let auth = e::owned(
        challenge.transcript(),
        2,
        e::ExpectedPre {
            stamp: &current.bytes,
            revision: 4,
            digest: e::hash(&prepared_envelope),
            head: oracle.head,
        },
        &payload,
        &mut audit,
    );
    assert_eq!(auth.sequence, 3);
    assert_eq!(challenge.digest(), e::hash(challenge.transcript()));
    assert_eq!(challenge.unsigned_payload(), payload);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    let settled_action = e::authenticated_action(&auth, &payload, &proof);
    let final_core = e::completed_core(
        &final_world,
        &[
            (&first, public.device, &first_batch),
            (&second, public.device, &batch),
        ],
    );
    let final_envelope = e::envelope(&final_core, &member, None);
    let final_known = KnownFrontiers {
        store_revision: 5,
        event_sequence: nf_contract::identity::EventSeq(2),
        ..source_known
    };
    let pre_rows = oracle.rows.clone();
    let pre_foot = oracle.foot();
    oracle.append(
        final_known,
        2,
        &settled_action,
        &final_envelope,
        Some(&batch),
    );
    expected.operations.insert(
        3,
        e::Operation {
            effect: e::physical_effect(2, 4, 4, &pre_rows, &oracle.rows, &pre_foot, &oracle.foot()),
            auth,
            action: settled_action,
            pre: pre_foot,
            post: oracle.foot(),
            pre_state: prepared_envelope,
            post_state: final_envelope.clone(),
        },
    );
    let ack = active.settle_current(settlement, proof).unwrap();
    assert_eq!(ack.sequence().0, 2);
    assert_eq!(
        final_world
            .view()
            .market(EntityId::from_bytes([30; 16]))
            .unwrap()
            .credits,
        51
    );
    assert_eq!(active.world(), &final_world);
    assert_eq!(active.core_bytes(), final_core);
    assert_eq!(active.envelope_bytes(), final_envelope);
    assert_eq!(active.known_frontiers(), final_known);
    assert_eq!(active.outbox().count(), 2);
    assert_eq!(active.storage_foot().canonical_bytes(), oracle.foot());
    let minimum = active.catalog_frontier();
    drop(active);
    case.check();
    oracle.observe(&case.target());
    e::replay_catalog(
        &case.catalog(),
        &member,
        &expected,
        &oracle.foot(),
        Some(&current),
        e::catalog_tuple(&minimum),
    );
    let mut restart =
        LocalCoordinator::open_existing(case.catalog(), installation, minimum).unwrap();
    let challenge = restart
        .issue_owner_challenge(&public, &oracle.foot(), case.cutoff())
        .unwrap();
    let restarted = e::prospective(
        challenge.transcript(),
        challenge.admission_input_bytes(),
        10,
        e::ProspectiveExpected {
            foot: &oracle.foot(),
            catalog: Some(e::catalog_tuple(&minimum)),
            public: &public,
            member: &member,
            paths: &[&catalog_path],
            composition: &[],
        },
        &mut audit,
    );
    e::assert_epoch(&restarted, 2, 4, 4, 4, to.digest());
    e::assert_claim_operation(challenge.admission_input_bytes(), &restarted);
    assert_ne!(restarted.incarnation, current.incarnation);
    let proof = e::sign(&public, &device_key, scope, 0, challenge.digest());
    expected.signed(challenge.transcript(), &proof);
    let active = restart.claim(proof).unwrap();
    assert_eq!(active.owner_stamp().canonical_bytes(), restarted.bytes);
    assert_eq!(active.world(), &final_world);
    assert_eq!(active.core_bytes(), final_core);
    assert_eq!(active.envelope_bytes(), final_envelope);
    assert_eq!(active.known_frontiers(), final_known);
    for (intent, sequence) in [(&first, 1), (&second, 2)] {
        let status = active
            .query_bound(intent.request, public.account, public.device, scope)
            .unwrap()
            .unwrap();
        assert!(
            matches!(status.status,nf_store::RequestStatus::Committed{operation,sequence:found,rejection:None} if found.0==sequence && operation==intent.operation)
        );
        assert_eq!(
            status.binding_digest,
            e::request_binding(intent, public.device)
        );
        assert!(status.administrative_cancellation.is_none());
    }
    let outbox = active.outbox().collect::<Vec<_>>();
    assert_eq!(outbox.len(), 2);
    for (intent, expected_batch) in [(&first, &first_batch), (&second, &batch)] {
        let item = outbox
            .iter()
            .find(|item| item.operation == intent.operation)
            .unwrap();
        assert_eq!(item.sequence, expected_batch.sequence);
        assert_eq!(
            item.batch_digest,
            e::hash(&nf_kernel::encode_batch(expected_batch).unwrap())
        );
        assert_eq!(item.rejection, None);
        assert!(item.administrative_cancellation.is_none());
    }
    let minimum = active.catalog_frontier();
    drop(active);
    oracle.observe(&case.target());
    e::observe_retired(&case.source(), &base, &retirement);
    e::replay_catalog(
        &case.catalog(),
        &member,
        &expected,
        &oracle.foot(),
        Some(&restarted),
        e::catalog_tuple(&minimum),
    );
    assert_eq!(e::bounded_read(&case.retained_source()), original);
    case.finish();
}
