//! Closed first-writer evidence, independent of the existing phase trace.
use super::super::super::writer_health::{self as health, FirstWriterHealth, HealthAdmission};
use super::*;
use std::io::{Seek, SeekFrom, Write};

#[test]
fn writer_health_accepts_only_complete_body_and_first_attempt_outcomes() {
    for (bytes, expected) in [
        (
            b"NFACLH1|1|BODY\n".as_slice(),
            FirstWriterHealth::BodyEntered,
        ),
        (
            b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_PUBLISHED\n".as_slice(),
            FirstWriterHealth::FirstPublished,
        ),
        (
            b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FAILED\n".as_slice(),
            FirstWriterHealth::FirstFailed,
        ),
    ] {
        assert_eq!(health::classify(bytes), (expected, PhaseRead::Observed));
    }
    assert_eq!(
        health::classify(b""),
        (FirstWriterHealth::Unknown, PhaseRead::Empty)
    );
    for bytes in [
        b"NFACLH1|1|BODY".as_slice(),
        b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FA",
        b"NFACLH1|2|FIRST_FAILED\n",
        b"NFACLH1|1|BODY\nNFACLH1|1|BODY\n",
        b"NFACLH1|1|BODY\nNFACLH1|3|FIRST_FAILED\n",
        b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FAILED\nNFACLH1|3|FIRST_PUBLISHED\n",
        b"NFACLH1|1|BODY\r\n",
        b"NFACLH1|01|BODY\n",
        b"NFACLH1|1|BODY|PRIVATE_PATH SID_SENTINEL\n",
        b"EXCEPTION_SENTINEL\n",
        &[0xff, b'\n'],
    ] {
        assert_eq!(
            health::classify(bytes),
            (FirstWriterHealth::Unknown, PhaseRead::Invalid)
        );
        let rendered = format!("{:?}", health::classify(bytes));
        for secret in ["PRIVATE_PATH", "SID_SENTINEL", "EXCEPTION_SENTINEL"] {
            assert!(!rendered.contains(secret));
        }
    }
    assert_eq!(
        health::classify(&[b'x'; 65]),
        (
            FirstWriterHealth::Unknown,
            PhaseRead::Failed(PhaseReadFailure::Oversize)
        )
    );
}

#[test]
fn writer_health_is_not_prepared_without_an_admitted_primary_capture() {
    let fixture = PhaseFixture::new();
    let (sidecar, admission) = health::prepare(
        &fixture.0.join("vault"),
        Some(&fixture.0.join("saves")),
        false,
    );
    assert!(sidecar.is_none());
    assert_eq!(admission, HealthAdmission::NotAttempted);
    let actual = health::observe(None, admission, true);
    assert_eq!(actual.observation, FirstWriterHealth::Unknown);
    assert_eq!(actual.read, PhaseRead::NotAttempted);
}

#[test]
fn writer_health_retains_its_own_file_until_terminal_and_does_not_seek_early() {
    let fixture = PhaseFixture::new();
    let mut sidecar = PhaseSidecar::in_directory(
        &fixture.0.join("observations"),
        &fixture.0.join("vault"),
        &fixture.0.join("saves"),
    )
    .unwrap();
    let path = sidecar.path().to_owned();
    sidecar
        .file
        .write_all(b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FAILED\n")
        .unwrap();
    sidecar.file.seek(SeekFrom::Start(3)).unwrap();
    let pending = health::observe(Some(&mut sidecar), HealthAdmission::Prepared, false);
    assert_eq!(pending.observation, FirstWriterHealth::Unknown);
    assert_eq!(pending.read, PhaseRead::TerminalUnconfirmed);
    assert_eq!(sidecar.file.stream_position().unwrap(), 3);
    let complete = health::observe(Some(&mut sidecar), HealthAdmission::Prepared, true);
    assert_eq!(complete.observation, FirstWriterHealth::FirstFailed);
    assert_eq!(complete.read, PhaseRead::Observed);
    drop(sidecar);
    assert!(!path.exists());
}

#[test]
fn writer_health_read_failure_preserves_independent_primary_phase_evidence() {
    let fixture = PhaseFixture::new();
    let make = || {
        PhaseSidecar::in_directory(
            &fixture.0.join("observations"),
            &fixture.0.join("vault"),
            &fixture.0.join("saves"),
        )
        .unwrap()
    };
    let mut primary = make();
    primary
        .file
        .write_all(b"NFACL1|1|SCRIPT_BODY_ENTERED\n")
        .unwrap();
    let mut sidecar = make();
    sidecar.file = std::fs::File::create(sidecar.path()).unwrap();
    let actual = health::observe(Some(&mut sidecar), HealthAdmission::Prepared, true);
    assert_eq!(actual.observation, FirstWriterHealth::Unknown);
    assert_eq!(
        actual.read,
        PhaseRead::Failed(PhaseReadFailure::ReadUnavailable)
    );
    assert_eq!(
        primary.observe_captured_after_terminal(true),
        (
            ChildPhaseObservation::Observed {
                last: HelperPhase::ScriptBodyEntered,
                records: 1
            },
            PhaseRead::Observed
        )
    );
}

#[test]
fn writer_health_rejects_oversize_owned_content_without_rendering_bytes() {
    let fixture = PhaseFixture::new();
    let mut sidecar = PhaseSidecar::in_directory(
        &fixture.0.join("observations"),
        &fixture.0.join("vault"),
        &fixture.0.join("saves"),
    )
    .unwrap();
    sidecar.file.write_all(&[b'x'; 65]).unwrap();
    let actual = health::observe(Some(&mut sidecar), HealthAdmission::Prepared, true);
    assert_eq!(actual.observation, FirstWriterHealth::Unknown);
    assert_eq!(actual.read, PhaseRead::Failed(PhaseReadFailure::Oversize));
}

#[test]
fn failed_publication_health_can_coexist_with_complete_or_partial_primary_bytes() {
    // A failed Flush may leave complete bytes. This is a closed evidence-pair
    // oracle, not a claim that the operating system produced a real flush fault.
    for bytes in [
        b"NFACL1|1|SCRIPT_BODY_ENTERED\n".as_slice(),
        b"NFACL1|1|SCRIPT_BODY",
    ] {
        let fixture = PhaseFixture::new();
        let mut primary = PhaseSidecar::in_directory(
            &fixture.0.join("observations"),
            &fixture.0.join("vault"),
            &fixture.0.join("saves"),
        )
        .unwrap();
        primary.file.write_all(bytes).unwrap();
        let observed = primary.observe_captured_after_terminal(true);
        assert_eq!(
            health::classify(b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FAILED\n").0,
            FirstWriterHealth::FirstFailed
        );
        assert_eq!(
            observed,
            if bytes.ends_with(b"\n") {
                (
                    ChildPhaseObservation::Observed {
                        last: HelperPhase::ScriptBodyEntered,
                        records: 1,
                    },
                    PhaseRead::Observed,
                )
            } else {
                (ChildPhaseObservation::Unknown, PhaseRead::Invalid)
            }
        );
    }
}
