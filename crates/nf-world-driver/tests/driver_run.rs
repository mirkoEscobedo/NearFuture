mod driver_support;
use nf_world_driver::{Driver, RunStop};
#[test]
fn foreground_run_pauses_without_activity_discards_offline_time_and_commits_only_one_hint_at_a_time()
 {
    let fixture = driver_support::Fixture::new();
    let mut driver = Driver::create(fixture.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let before = driver.status().unwrap();
    let paused = driver
        .run_for(100, 1, false)
        .expect("inactive foreground pauses");
    assert_eq!(paused.stop, RunStop::Inactive);
    assert_eq!(paused.committed_ticks, 0);
    assert_eq!(driver.status().unwrap(), before);
    assert!(driver.run_for(99, 1, true).is_err());
    assert!(driver.run_for(100, 61, true).is_err());
    let none = driver.run_for(60000, 1, true).unwrap();
    assert_eq!(none.stop, RunStop::Deadline);
    assert_eq!(none.committed_ticks, 0);
    assert_eq!(driver.status().unwrap(), before);
    let run = driver.run_for(100, 1, true).unwrap();
    assert_eq!(run.stop, RunStop::Deadline);
    assert!(run.committed_ticks > 0 && run.committed_ticks <= 9);
    let after = driver.status().unwrap();
    assert_eq!(after.world.metadata().tick.0, run.committed_ticks);
    assert_eq!(
        after.world.metadata().provider_revision.0,
        run.committed_ticks
    );
    assert_eq!(after.world.component(), before.world.component());
    // Each new window starts from its own monotonic origin; no saved wall-time catch-up.
    let unchanged = driver.run_for(60000, 1, true).unwrap();
    assert_eq!(unchanged.committed_ticks, 0);
    assert_eq!(driver.status().unwrap(), after);
}
