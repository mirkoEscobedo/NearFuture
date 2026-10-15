#[path = "diagnostic_stream_observation/mod.rs"]
mod support;

use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    time::Instant,
};
use support::{GOLDEN, custody::{Evidence, Stage}};

#[test]
#[ignore = "requires caller-owned public diagnostic observation evidence"]
fn non_draining_redacted_diagnostic_observation_preserves_fifo_and_unverified_scope() {
    let started = Instant::now();
    let mut owned = None;
    let result = catch_unwind(AssertUnwindSafe(|| {
        Evidence::create(started, &mut owned);
        let evidence = owned.as_mut().expect("owned case required");
        let originals = evidence.run(Stage::RamFixture, || support::records(0));
        let transformed = evidence.run(Stage::RamFixture, || support::records(0x80));
        evidence.enter(Stage::FixtureCapture);
        evidence.write("expected.literal.json", GOLDEN.as_bytes());
        evidence.write("original-public-fixture.bin", &support::fixture_bytes(&originals));
        evidence.write("xor-public-fixture.bin", &support::fixture_bytes(&transformed));
        evidence.enter(Stage::BaselineSnapshot);
        let before = evidence.snapshot();
        let mut stream = evidence.run(Stage::RamFixture, || support::stream(&originals));
        let mut control = evidence.run(Stage::RamFixture, || support::stream(&transformed));
        let first = evidence.run(Stage::OriginalObservation, || stream.redacted_observation());
        let second = evidence.run(Stage::RepeatedObservation, || stream.redacted_observation());
        let private_control = evidence.run(Stage::PrivacyObservation, || control.redacted_observation());
        let projections = [
            evidence.run(Stage::OriginalProjection, || support::project(&first)),
            evidence.run(Stage::RepeatedProjection, || support::project(&second)),
            evidence.run(Stage::PrivacyProjection, || support::project(&private_control)),
        ];
        let debug = [
            evidence.run(Stage::OriginalDebug, || format!("{first:?}")),
            evidence.run(Stage::RepeatedDebug, || format!("{second:?}")),
            evidence.run(Stage::PrivacyDebug, || format!("{private_control:?}")),
        ];
        let popped = [
            evidence.run(Stage::OriginalFifo, || stream.pop()),
            evidence.run(Stage::OriginalFifo, || stream.pop()),
            evidence.run(Stage::OriginalFifo, || stream.pop()),
            evidence.run(Stage::OriginalFifo, || stream.pop()),
        ];
        let control_popped = [
            evidence.run(Stage::PrivacyFifo, || control.pop()),
            evidence.run(Stage::PrivacyFifo, || control.pop()),
            evidence.run(Stage::PrivacyFifo, || control.pop()),
            evidence.run(Stage::PrivacyFifo, || control.pop()),
        ];
        let drops = [
            evidence.run(Stage::OriginalFifo, || stream.dropped()),
            evidence.run(Stage::PrivacyFifo, || control.dropped()),
        ];
        evidence.enter(Stage::TreePreservation);
        assert_eq!(evidence.snapshot(), before, "observation altered retained originals");
        evidence.enter(Stage::OutputCapture);
        for (index, projection) in projections.iter().enumerate() {
            evidence.write(&format!("observation-{index}.json"), projection.as_bytes());
            evidence.write(&format!("observation-{index}.debug.txt"), debug[index].as_bytes());
        }
        evidence.write("original-popped.bin", &support::popped_bytes(&popped));
        evidence.write("xor-popped.bin", &support::popped_bytes(&control_popped));
        evidence.write("drop-counts.txt", format!("{} {}\n", drops[0], drops[1]).as_bytes());
        evidence.enter(Stage::LiteralAssertions);
        assert_eq!(GOLDEN.len(), 472);
        assert_eq!(support::sha256(GOLDEN.as_bytes()), "1a2e07ada58753bde0fa84e722c3b083235ce15ac59ca75dcf28d54ae9e01752");
        for projection in &projections {
            assert_eq!(projection, GOLDEN, "independent closed literal mismatch");
        }
        evidence.enter(Stage::PrivacyAssertions);
        for (projection, formatted) in projections.iter().zip(&debug) {
            assert_eq!(formatted, &debug[0], "private substitution changed safe Debug");
            support::assert_private_values_absent(projection, &originals, &transformed);
            support::assert_private_values_absent(formatted, &originals, &transformed);
        }
        evidence.enter(Stage::FifoAssertions);
        for index in 0..3 {
            assert_eq!(popped[index].as_ref(), Some(&originals[index]));
            assert_eq!(control_popped[index].as_ref(), Some(&transformed[index]));
        }
        assert_eq!(popped[3], None);
        assert_eq!(control_popped[3], None);
        assert_eq!(drops, [1, 1]);
        evidence.enter(Stage::CompletionCheck);
    }));
    let retained = catch_unwind(AssertUnwindSafe(|| {
        if let Some(evidence) = owned.as_mut() {
            evidence.finish(result.is_ok());
        } else {
            eprintln!("INSUFFICIENT_EVIDENCE: no owned case namespace; caller receipt only");
        }
    }));
    if let Err(original) = result {
        if retained.is_err() {
            eprintln!("INSUFFICIENT_EVIDENCE: bounded final custody failed; originals retained");
        }
        resume_unwind(original);
    }
    if let Err(custody) = retained {
        resume_unwind(custody);
    }
}
