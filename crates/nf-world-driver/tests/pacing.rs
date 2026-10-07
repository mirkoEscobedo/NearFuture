use nf_world_driver::{Pacer, PacingError};

#[test]
fn a_long_gap_issues_one_hint_and_commit_rearm_discards_missed_slots() {
    let mut p = Pacer::new(1_000, 0).unwrap();
    assert!(p.observe(999).unwrap().is_none());
    let hint = p
        .observe(300_000)
        .unwrap()
        .expect("one due hint after five-minute gap");
    assert_eq!((hint.deadline_ms(), hint.observed_ms()), (1_000, 300_000));
    for now in [300_000, 301_000, 600_000] {
        assert!(p.observe(now).unwrap().is_none());
    }
    p.rearm(600_000).unwrap();
    assert!(p.observe(600_999).unwrap().is_none());
    assert_eq!(p.observe(601_000).unwrap().unwrap().deadline_ms(), 601_000);
    assert!(p.observe(1_000_000).unwrap().is_none());
}
#[test]
fn pause_clears_a_pending_hint_and_resume_schedules_from_the_new_observation() {
    let mut p = Pacer::new(100, 10).unwrap();
    assert!(p.observe(110).unwrap().is_some());
    p.pause(110).unwrap();
    for now in [110, 1_000, 1_000_000] {
        assert!(p.observe(now).unwrap().is_none());
    }
    p.rearm(1_000_000).unwrap();
    assert!(p.observe(1_000_099).unwrap().is_none());
    assert!(p.observe(1_000_100).unwrap().is_some());
}
#[test]
fn bad_observation_poisoning_prevents_stale_deadline_recovery_through_any_method() {
    for (trigger, phase) in (0..3).flat_map(|trigger| (0..3).map(move |phase| (trigger, phase))) {
        let mut p = Pacer::new(100, 200).unwrap();
        if phase == 1 {
            assert!(p.observe(300).unwrap().is_some());
        } else if phase == 2 {
            p.pause(300).unwrap();
        }
        let error = match trigger {
            0 => p.observe(199).map(|_| ()),
            1 => p.pause(199),
            _ => p.rearm(199),
        };
        assert_eq!(error, Err(PacingError::BackwardsObservation));
        assert_eq!(p.observe(500), Err(PacingError::Poisoned));
        assert_eq!(p.pause(500), Err(PacingError::Poisoned));
        assert_eq!(p.rearm(500), Err(PacingError::Poisoned));
    }
    assert!(
        Pacer::new(100, 500)
            .unwrap()
            .observe(600)
            .unwrap()
            .is_some()
    );
}
#[test]
fn period_bounds_and_checked_deadlines_do_not_wrap_or_reuse_an_old_hint() {
    for period in [0, 1, 99, 60_001, u64::MAX] {
        assert!(matches!(
            Pacer::new(period, 0),
            Err(PacingError::InvalidPeriod)
        ));
    }
    for period in [100, 60_000] {
        let mut p = Pacer::new(period, u64::MAX - period).unwrap();
        assert!(p.observe(u64::MAX - 1).unwrap().is_none());
        assert!(p.observe(u64::MAX).unwrap().is_some());
        assert_eq!(p.rearm(u64::MAX), Err(PacingError::Overflow));
        assert_eq!(p.observe(u64::MAX), Err(PacingError::Poisoned));
    }
    assert!(matches!(
        Pacer::new(100, u64::MAX - 99),
        Err(PacingError::Overflow)
    ));
}
#[test]
fn finite_period_and_gap_corpus_has_no_implicit_repeated_due_hints() {
    for period in [100, 101, 999, 1_000, 60_000] {
        for gap in [0, 1, 99, 100, 101, 60_000, 300_000, 1_000_000] {
            let mut p = Pacer::new(period, 0).unwrap();
            assert_eq!(p.observe(gap).unwrap().is_some(), gap >= period);
            if gap >= period {
                assert!(p.observe(gap + period * 10).unwrap().is_none());
            }
            let now = gap + period * 10;
            p.rearm(now).unwrap();
            assert!(p.observe(now + period - 1).unwrap().is_none());
            assert!(p.observe(now + period).unwrap().is_some());
            assert!(p.observe(now + period * 2).unwrap().is_none());
        }
    }
}
