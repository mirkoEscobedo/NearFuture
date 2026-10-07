mod driver_support;
use nf_contract::identity::WorldTick;
use nf_world_driver::{Driver, RunStage, RunStop};
#[test]
fn callback_latency_after_real_prepare_expires_window_before_new_advancing_commit_and_next_window_can_resume()
 {
    let f = driver_support::Fixture::new();
    let mut driver = Driver::create(f.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let initial = driver.status().unwrap();
    let mut observed = false;
    let report = driver
        .run_for_with_observer(100, 1, true, &mut |stage| {
            assert_eq!(stage, RunStage::PendingPrepared);
            observed = true;
            std::thread::sleep(std::time::Duration::from_millis(1200));
        })
        .unwrap();
    assert!(
        observed,
        "actual authenticated SQL Prepare reached read-only progress port"
    );
    assert_eq!(report.stop, RunStop::Deadline);
    let paused = driver.status().unwrap();
    assert_eq!(
        paused.world.metadata().tick,
        WorldTick(0),
        "expired window cannot admit a new advancing COMMIT"
    );
    assert_eq!(report.committed_ticks, 0);
    assert!(paused.pending.is_some());
    assert_eq!(paused.world, initial.world);
    assert!(
        paused.known.storage.store_revision > initial.known.storage.store_revision,
        "real Prepare remains durable"
    );
    driver.advance_pending(true).expect("finished run cleared its private deadline; explicit fresh activity can commit retained work");
    assert_eq!(driver.status().unwrap().world.metadata().tick, WorldTick(1));
    assert_eq!(driver.run_for(60000, 1, true).unwrap().committed_ticks, 0);
}
#[test]
fn failed_run_clears_private_deadline_before_later_explicit_authority_claim() {
    let f = driver_support::Fixture::new();
    let mut driver = Driver::create(f.options.clone()).unwrap();
    assert!(
        driver.run_for(100, 1, true).is_err(),
        "run_for never creates a hidden claim"
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    driver.claim_authority().expect(
        "error return cleared only the driver budget; actual Store proof grants the new claim",
    );
    assert_eq!(driver.status().unwrap().world.metadata().tick, WorldTick(0));
    assert_eq!(driver.run_for(60000, 1, true).unwrap().committed_ticks, 0);
}
