#[path = "actual_producer_metric_capacity/mod.rs"]
mod support;
use near_future::application::{
    ForegroundRequest, PackageObservationScopeV1, PortableAdmissionInputV1, start_foreground,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    time::Instant,
};
use support::{
    Brackets, EXPECTED,
    custody::{Evidence, Stage},
    evaluate, fixture, projection, write,
};

fn exercise(evidence: &mut Evidence) {
    let (metadata, input, reference, original) = evidence.run(Stage::RamFixture, || {
        let (metadata, input, reference) = fixture::fixture();
        let original = fixture::public_original(&metadata, &input, &reference);
        (metadata, input, reference, original)
    });
    evidence.enter(Stage::FixtureCapture);
    write(evidence, "public-ram-fixture.original.txt", &original);
    write(
        evidence,
        "expected.capacity-selection.json",
        EXPECTED.as_bytes(),
    );
    evidence.enter(Stage::BaselineSnapshot);
    let baseline = evidence.snapshot();
    assert_eq!(baseline.len(), 3, "closed original fixture tree");
    let mut host = evidence
        .run(Stage::StartHost, || {
            start_foreground(ForegroundRequest {
                lifetime_millis: 60_000,
                package_scope: PackageObservationScopeV1::WorkspacePublicSubset,
            })
        })
        .unwrap_or_else(|_| panic!("actual foreground startup refused"));
    evidence
        .run(Stage::RegisterSession, || {
            host.register_portable_session(&metadata, PortableAdmissionInputV1::Action(&input))
        })
        .unwrap_or_else(|_| panic!("actual canonical registration refused"));
    let empty = evidence.run(Stage::EmptyObservation, || {
        projection::observe(&mut host, true, true)
    });
    let empty = evidence.run(Stage::EmptyProjection, || {
        projection::snapshot(&empty, 0, 0)
    });
    let mut brackets = Brackets::new();
    for _ in 0..64 {
        evaluate(evidence, &mut host, &input, &reference, &mut brackets);
    }
    let maximum64 = evidence.run(Stage::MetricProjection, || brackets.maximum());
    let owned64 = evidence.run(Stage::FirstObservation, || {
        projection::observe(&mut host, true, true)
    });
    let full = evidence.run(Stage::MetricProjection, || {
        projection::snapshot(&owned64, 64, maximum64)
    });
    evaluate(evidence, &mut host, &input, &reference, &mut brackets);
    let maximum65 = evidence.run(Stage::MetricProjection, || brackets.maximum());
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
    let (first, second, omitted, last, snapshot64_unchanged) =
        evidence.run(Stage::MetricProjection, || {
            (
                projection::snapshot(&first, 65, maximum65),
                projection::snapshot(&second, 65, maximum65),
                projection::omitted(&omitted),
                projection::snapshot(&last, 65, maximum65),
                projection::snapshot(&owned64, 64, maximum64) == full,
            )
        });
    let exit = evidence
        .run(Stage::StopHost, || host.stop())
        .unwrap_or_else(|_| panic!("actual foreground stop refused"));
    evidence.enter(Stage::TreePreservation);
    let tree_unchanged = evidence.snapshot() == baseline;
    let after = evidence.run(Stage::TreePreservation, || {
        fixture::public_original(&metadata, &input, &reference)
    });
    let originals_unchanged = tree_unchanged && after == original;
    assert!(originals_unchanged, "product changed owned originals");
    let (actual, files) = evidence.run(Stage::OutputCapture, || {
        projection::captures(
            [&empty, &full, &first, &second, &last],
            &omitted,
            &brackets,
            snapshot64_unchanged,
            originals_unchanged,
            exit,
            &after,
        )
    });
    for (name, bytes) in files {
        write(evidence, &name, bytes.as_bytes());
    }
    evidence.run(Stage::LiteralAssertions, || {
        assert_eq!(actual.as_bytes(), EXPECTED.as_bytes());
        assert_eq!(brackets.len(), 65);
        for value in [&empty, &full, &first, &second, &last] {
            fixture::assert_private_bindings_absent(&value.stable, &metadata);
            fixture::assert_private_bindings_absent(&value.actual(), &metadata);
        }
        fixture::assert_private_bindings_absent(&actual, &metadata);
        fixture::assert_private_bindings_absent(&omitted.actual, &metadata);
        assert!(!actual.contains("input_digest"));
    });
    evidence.enter(Stage::CompletionCheck);
}

#[test]
#[ignore = "requires an explicit existing caller-owned public evidence parent"]
fn actual_metric_capacity_retains_64_of_65_real_evaluations_without_observation_samples() {
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
    assert!(captured, "bounded retained capacity finalization failed");
}
