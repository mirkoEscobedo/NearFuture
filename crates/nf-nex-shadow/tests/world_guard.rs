mod world_support;
use nf_nex_boundary::NexWorld;
use nf_nex_shadow::*;
#[test]
fn same_admitted_world_can_compare_shadow_diagnostics_and_unused_field_drift_rejects() {
    let s = world_support::snapshot();
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let world = NexWorld::admit(s.clone()).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &world).unwrap();
    let result = session
        .evaluate_war(&world, WarOperation::Update, target)
        .unwrap();
    assert!(
        session
            .accept_war(&result, &m, &world, WarOperation::Update, target)
            .is_ok()
    );
    for mutation in 0..4 {
        let mut drift = s.clone();
        match mutation {
            0 => drift.weariness.raw = nf_nex_boundary::FloatBits::new(1).unwrap(),
            1 => drift.timers.meeting += 1,
            2 => drift.concerns[0].cooldown = nf_nex_boundary::FloatBits::new(0).unwrap(),
            3 => {
                drift.concern_config.anti_repetition_multiplier =
                    nf_nex_boundary::FloatBits::new(1).unwrap()
            }
            _ => unreachable!(),
        }
        let changed = NexWorld::admit(drift).unwrap();
        assert_eq!(
            war_from_world(&world, WarOperation::Update, target),
            war_from_world(&changed, WarOperation::Update, target)
        );
        assert_eq!(
            session.accept_war(&result, &m, &changed, WarOperation::Update, target),
            Err(Unavailable::Stale)
        );
    }
}
#[test]
fn named_existing_instance_does_not_require_auto_generation_permission() {
    let mut s = world_support::snapshot();
    s.concern_config.no_auto_generate = true;
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let world = NexWorld::admit(s).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &world).unwrap();
    let result = session
        .evaluate_war(&world, WarOperation::Update, target)
        .unwrap();
    assert!(
        session
            .accept_war(&result, &m, &world, WarOperation::Update, target)
            .is_ok()
    );
    assert_eq!(
        session.evaluate_war(&world, WarOperation::Generate, None),
        Err(Unavailable::Disabled)
    );
    assert_eq!(
        session.accept_war(&result, &m, &world, WarOperation::Update, target),
        Err(Unavailable::Disabled)
    );
}
#[test]
fn full_owner_method_target_and_lifecycle_are_checked_at_acceptance() {
    use nf_contract::identity::*;
    let s = world_support::snapshot();
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let w = NexWorld::admit(s).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &w).unwrap();
    let result = session
        .evaluate_war(&w, WarOperation::Update, target)
        .unwrap();
    for mutation in 0..17 {
        let mut current = m.clone();
        match mutation {
            0 => current.provenance.runtime_session.0 += 1,
            1 => current.provenance.frontier.0 += 1,
            2 => current.provenance.source_digest[0] ^= 1,
            3 => current.provenance.runtime_digest[0] ^= 1,
            4 => current.provenance.ruleset_digest[0] ^= 1,
            5 => current.provenance.merged_config_digest[0] ^= 1,
            6 => current.reference_digest[0] ^= 1,
            7 => current.corpus_digest[0] ^= 1,
            8 => current.implementation_digest[0] ^= 1,
            9 => current.capture_policy_digest[0] ^= 1,
            10 => current.provider = ProviderId::from_bytes([30; 16]),
            11 => current.campaign = CampaignId::from_bytes([31; 16]),
            12 => current.branch = BranchId::from_bytes([32; 16]),
            13 => current.subject_faction = "tritachyon".into(),
            14 => current.concern_instance = None,
            15 => current.provenance.universe = UniverseId::from_bytes([33; 16]),
            16 => current.provenance.history = HistoryId::from_bytes([34; 16]),
            _ => unreachable!(),
        }
        assert_eq!(
            session.accept_war(&result, &current, &w, WarOperation::Update, target),
            Err(Unavailable::Stale),
            "owner {mutation}"
        );
    }
    {
        let method = WarOperation::IsValid;
        assert_eq!(
            session.accept_war(&result, &m, &w, method, target),
            Err(Unavailable::Stale)
        );
    }
    assert_eq!(
        session.accept_war(&result, &m, &w, WarOperation::Generate, None),
        Err(Unavailable::Stale)
    );
    let mut invalid = m.clone();
    invalid.provenance.runtime_session.0 = 0;
    assert_eq!(
        session.accept_war(&result, &invalid, &w, WarOperation::Update, target),
        Err(Unavailable::MissingFact)
    );
    session.invalidate();
    assert_eq!(
        session.accept_war(&result, &m, &w, WarOperation::Update, target),
        Err(Unavailable::Disabled)
    );
    let mut next = m;
    next.provenance.runtime_session.0 += 1;
    let mut s = w.snapshot().clone();
    s.provenance = next.provenance.clone();
    let next_world = NexWorld::admit(s).unwrap();
    let fresh = WorldShadowSession::new(next.clone(), &next_world).unwrap();
    assert_eq!(
        fresh.accept_war(&result, &next, &next_world, WarOperation::Update, target),
        Err(Unavailable::Stale)
    );
}
#[test]
fn observed_world_drift_disables_old_session_and_legacy_world_guard_stays_closed() {
    let s = world_support::snapshot();
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let w = NexWorld::admit(s.clone()).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &w).unwrap();
    let result = session
        .evaluate_war(&w, WarOperation::Update, target)
        .unwrap();
    let mut changed = s;
    changed.timers.meeting += 1;
    let changed = NexWorld::admit(changed).unwrap();
    assert_eq!(
        session.evaluate_war(&changed, WarOperation::Update, target),
        Err(Unavailable::Stale)
    );
    assert_eq!(
        session.accept_war(&result, &m, &w, WarOperation::Update, target),
        Err(Unavailable::Disabled)
    );
    let old = evaluate_world_war(&m, &w, WarOperation::Update, target).unwrap();
    let input = ShadowInput::War {
        operation: WarOperation::Update,
        facts: war_from_world(&w, WarOperation::Update, target).unwrap(),
    };
    assert_eq!(
        ShadowSession::new(m.clone()).accept(&old, &m, &input),
        Err(Unavailable::MissingFact)
    );
    assert_eq!(selected_peace_from_world(&w), Err(Unavailable::MissingFact));
}
#[test]
fn generation_never_binds_an_existing_concern_instance() {
    let s = world_support::snapshot();
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let w = NexWorld::admit(s).unwrap();
    let mut named = WorldShadowSession::new(m, &w).unwrap();
    assert_eq!(
        named.evaluate_war(&w, WarOperation::Generate, target),
        Err(Unavailable::Unsupported)
    );
}
#[test]
fn generation_uses_none_and_updates_require_the_exact_existing_target() {
    let s = world_support::snapshot();
    let mut m = world_support::metadata(&s);
    m.concern_instance = None;
    let w = NexWorld::admit(s).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &w).unwrap();
    let r = session
        .evaluate_war(&w, WarOperation::Generate, None)
        .unwrap();
    assert!(
        session
            .accept_war(&r, &m, &w, WarOperation::Generate, None)
            .is_ok()
    );
    assert_eq!(
        session.accept_war(&r, &m, &w, WarOperation::Update, None),
        Err(Unavailable::MissingFact)
    );
    assert_eq!(
        session.accept_war(
            &r,
            &m,
            &w,
            WarOperation::Generate,
            Some(nf_contract::identity::EntityId::from_bytes([9; 16]))
        ),
        Err(Unavailable::Unsupported)
    );
}
#[test]
fn captured_unverified_world_comparison_never_promotes_authority() {
    let mut s = world_support::snapshot();
    s.provenance.observation = nf_nex_boundary::Observation::CapturedUnverified;
    let m = world_support::metadata(&s);
    let target = m.concern_instance;
    let w = NexWorld::admit(s).unwrap();
    let mut session = WorldShadowSession::new(m.clone(), &w).unwrap();
    let result = session
        .evaluate_war(&w, WarOperation::Update, target)
        .unwrap();
    assert!(
        session
            .accept_war(&result, &m, &w, WarOperation::Update, target)
            .is_ok()
    );
    assert_eq!(result.authority(), ShadowAuthority::ShadowOnly);
    assert_eq!(
        w.assessment().runtime,
        nf_nex_boundary::RuntimeCertification::Unobserved
    );
}
