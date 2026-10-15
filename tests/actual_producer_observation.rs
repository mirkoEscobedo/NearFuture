#[path = "actual_producer_observation/mod.rs"]
mod support;
use near_future::application::{
    ForegroundRequest, HostExit, PackageObservationScopeV1, PortableAdmissionInputV1,
    ProducerObservationSelection, ProducerScopeV1, start_foreground,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    time::Instant,
};
use support::{
    OUTER, QUEUE,
    custody::{Evidence, Stage},
    fixture, projection,
};

fn exercise(evidence: &mut Evidence) {
    let (metadata, input, reference) = evidence.run(Stage::RamFixture, fixture::fixture);
    let original = fixture::public_original(&metadata, &input, &reference);
    evidence.enter(Stage::FixtureCapture);
    evidence.write("public-ram-fixture.original.txt", &original);
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
    let receipt = evidence
        .run(Stage::EvaluateAction, || {
            host.evaluate_action_copied(&input, &reference)
        })
        .unwrap_or_else(|_| panic!("actual evaluation refused"));
    evidence.run(Stage::ReceiptAssertions, || {
        projection::assert_receipt(&receipt)
    });
    let first = evidence
        .run(Stage::FirstObservation, || {
            host.observe_producer(ProducerObservationSelection {
                required_scope: ProducerScopeV1::PortableCopiedFactsSinceOwnerStart,
                include_l1: true,
                include_metrics: false,
            })
        })
        .unwrap_or_else(|_| panic!("first actual owner observation refused"));
    let second = evidence
        .run(Stage::SecondObservation, || {
            host.observe_producer(ProducerObservationSelection {
                required_scope: ProducerScopeV1::PortableCopiedFactsSinceOwnerStart,
                include_l1: true,
                include_metrics: false,
            })
        })
        .unwrap_or_else(|_| panic!("second actual owner observation refused"));
    let first_bytes = evidence.run(Stage::FirstProjection, || projection::project(&first));
    let second_bytes = evidence.run(Stage::SecondProjection, || projection::project(&second));
    let exit = evidence
        .run(Stage::StopHost, || host.stop())
        .unwrap_or_else(|_| panic!("actual foreground stop refused"));
    assert!(matches!(exit, HostExit::Stopped));

    evidence.enter(Stage::TreePreservation);
    assert_eq!(
        evidence.snapshot(),
        baseline,
        "product changed owned public originals"
    );
    assert_eq!(
        fixture::public_original(&metadata, &input, &reference),
        original
    );
    evidence.enter(Stage::OutputCapture);
    evidence.write("actual.first.outer.json", first_bytes.0.as_bytes());
    evidence.write("actual.first.queue.json", first_bytes.1.as_bytes());
    evidence.write("actual.second.outer.json", second_bytes.0.as_bytes());
    evidence.write("actual.second.queue.json", second_bytes.1.as_bytes());
    evidence.write(
        "actual.typed-receipt.txt",
        b"shadow_only portable_copied_facts_since_owner_start mismatched nex_decision_mismatch\n",
    );
    evidence.enter(Stage::LiteralAssertions);
    assert_eq!(OUTER.len(), 487);
    assert_eq!(QUEUE.len(), 163);
    for (outer, queue) in [&first_bytes, &second_bytes] {
        assert_eq!(outer.as_bytes(), OUTER.as_bytes());
        assert_eq!(queue.as_bytes(), QUEUE.as_bytes());
        fixture::assert_private_bindings_absent(outer, &metadata);
        fixture::assert_private_bindings_absent(queue, &metadata);
    }
    assert_eq!(
        first_bytes, second_bytes,
        "same actual queue observation drained or drifted"
    );
    evidence.enter(Stage::CompletionCheck);
}

#[test]
#[ignore = "requires an explicit existing caller-owned public evidence parent"]
fn actual_portable_producer_observation_tracks_real_comparison_and_preserves_owned_queue() {
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
