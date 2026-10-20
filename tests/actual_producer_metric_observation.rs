#[path = "actual_producer_metric_observation/mod.rs"]
mod support;
use near_future::application::{
    ForegroundRequest, HostExit, PackageObservationScopeV1, PortableAdmissionInputV1,
    start_foreground,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    time::Instant,
};
use support::{
    METRIC, OUTER, QUEUE,
    custody::{Evidence, Stage},
    fixture, projection,
};

fn exercise(evidence: &mut Evidence) {
    let (metadata, input, reference, original) = evidence.run(Stage::RamFixture, || {
        let (metadata, input, reference) = fixture::fixture();
        let original = fixture::public_original(&metadata, &input, &reference);
        (metadata, input, reference, original)
    });
    evidence.enter(Stage::FixtureCapture);
    evidence.write("public-ram-fixture.original.txt", &original);
    evidence.write("expected.metrics.json", METRIC.as_bytes());
    evidence.write("expected.outer.json", OUTER.as_bytes());
    evidence.write("expected.queue.json", QUEUE.as_bytes());
    evidence.enter(Stage::BaselineSnapshot);
    let baseline = evidence.snapshot();
    let request = ForegroundRequest {
        lifetime_millis: 60_000,
        package_scope: PackageObservationScopeV1::WorkspacePublicSubset,
    };
    let mut host = evidence
        .run(Stage::StartHost, || start_foreground(request))
        .unwrap_or_else(|_| panic!("actual foreground startup refused"));
    evidence
        .run(Stage::RegisterSession, || {
            host.register_portable_session(&metadata, PortableAdmissionInputV1::Action(&input))
        })
        .unwrap_or_else(|_| panic!("actual canonical registration refused"));
    let empty = evidence.run(Stage::EmptyObservation, || {
        projection::observe(&mut host, true, true)
    });
    let empty_bytes = evidence.run(Stage::EmptyProjection, || projection::empty(&empty));
    let (receipt, enclosing) = evidence.run(Stage::EvaluateAction, || {
        let before = Instant::now();
        let result = host.evaluate_action_copied(&input, &reference);
        let after = Instant::now();
        (result, after.checked_duration_since(before))
    });
    let receipt = receipt.unwrap_or_else(|_| panic!("actual accepted evaluation required"));
    let enclosing_ns = enclosing.expect("real caller monotonic bracket").as_nanos();
    evidence.run(Stage::ReceiptAssertions, || {
        projection::assert_receipt(&receipt)
    });
    let first = evidence.run(Stage::FirstObservation, || {
        projection::observe(&mut host, true, true)
    });
    let second = evidence.run(Stage::SecondObservation, || {
        projection::observe(&mut host, true, true)
    });
    let omitted = evidence.run(Stage::NotSelectedObservation, || {
        projection::observe(&mut host, false, false)
    });
    let last = evidence.run(Stage::FinalObservation, || {
        projection::observe(&mut host, true, true)
    });
    let (first_metric, second_metric, last_metric, first_public, second_public, last_public) =
        evidence.run(Stage::MetricProjection, || {
            projection::not_selected(&omitted);
            (
                projection::one(&first, enclosing_ns),
                projection::one(&second, enclosing_ns),
                projection::one(&last, enclosing_ns),
                projection::project(&first),
                projection::project(&second),
                projection::project(&last),
            )
        });
    let exit = evidence
        .run(Stage::StopHost, || host.stop())
        .unwrap_or_else(|_| panic!("actual foreground stop refused"));
    assert!(matches!(exit, HostExit::Stopped));
    evidence.enter(Stage::TreePreservation);
    assert_eq!(
        evidence.snapshot(),
        baseline,
        "product changed owned originals"
    );
    evidence.run(Stage::TreePreservation, || {
        assert_eq!(
            fixture::public_original(&metadata, &input, &reference),
            original
        );
    });
    let metrics = [&first_metric, &second_metric, &last_metric];
    let public = [&first_public, &second_public, &last_public];
    let captures = evidence.run(Stage::OutputCapture, || {
        projection::captures(metrics, public, &empty_bytes)
    });
    for (name, bytes) in captures {
        evidence.write(&name, bytes.as_bytes());
    }
    evidence.run(Stage::LiteralAssertions, || {
        projection::assert_retained(metrics, public, &empty_bytes, &metadata)
    });
    evidence.enter(Stage::CompletionCheck);
}

#[test]
#[ignore = "requires an explicit existing caller-owned public evidence parent"]
fn actual_metric_window_observes_one_evaluation_without_observation_samples() {
    let started = Instant::now();
    let mut owned = None;
    let result = catch_unwind(AssertUnwindSafe(|| {
        Evidence::create(started, &mut owned);
        exercise(owned.as_mut().unwrap());
    }));
    let passed = result.is_ok();
    let captured = match owned.as_mut() {
        Some(evidence) => catch_unwind(AssertUnwindSafe(|| evidence.finish(passed))).is_ok(),
        None => false,
    };
    if let Err(payload) = result {
        resume_unwind(payload);
    }
    assert!(captured, "bounded retained case finalization failed");
}
