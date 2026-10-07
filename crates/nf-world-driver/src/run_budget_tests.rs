//! Real backend regression for the private rejection-only transaction admission cut.
#[path = "../tests/driver_support/mod.rs"]
mod driver_support;
use crate::{Driver, run_budget::deadline_hook};
use nf_contract::identity::*;
use nf_kernel::miniature::MiniatureIntent;
use nf_store::{
    Boundary,
    miniature::{ChallengeRequest, MiniatureStoreError},
};
use std::time::{Duration, Instant};
#[test]
fn delayed_after_caller_budget_check_cannot_start_sql_writes_at_expired_transaction_entry() {
    let fixture = driver_support::Fixture::new();
    let mut driver = Driver::create(fixture.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let world = driver.store.world();
    let g = world.component().genesis();
    let m = world.metadata();
    let public = driver.signer.public();
    let intent = MiniatureIntent {
        request: RequestId::from_bytes([40; 16]),
        operation: OperationId::from_bytes([41; 16]),
        job: JobId::from_bytes([42; 16]),
        actor: public.account,
        device: public.device,
        universe: g.universe,
        history: g.history,
        provider: m.provider,
        expected: [
            (m.aggregate, world.component().revision()),
            (m.provider_aggregate, m.provider_revision),
        ]
        .into(),
        action: crate::resolve_action(
            world,
            &crate::ActionSelection::Build {
                market: 0,
                kind: nf_world::IndustryKind::Farming,
            },
        )
        .unwrap(),
    };
    let proof = driver.proof(ChallengeRequest::Prepare(&intent)).unwrap();
    let before = driver.status().unwrap();
    driver.run_deadline = Some(Instant::now() + Duration::from_millis(150));
    driver.check_budget().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let mut guard = deadline_hook(driver.run_deadline);
    let mut after_writes = false;
    let result = driver
        .store
        .prepare_with_hook(vec![intent], vec![proof], &mut |boundary| {
            guard(boundary)?;
            if boundary == Boundary::AfterWrites {
                after_writes = true;
            }
            Ok(())
        });
    assert_eq!(result.err(), Some(MiniatureStoreError::Expired));
    assert!(
        !after_writes,
        "an expired transaction must be refused before real SQL writes, not only rolled back at BeforeCommit"
    );
    assert_eq!(driver.status().unwrap(), before);
    driver.run_deadline = None;
}
