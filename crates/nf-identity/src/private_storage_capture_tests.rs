use super::{PhaseFixture, *};
use std::io::{Seek, SeekFrom, Write};

#[test]
fn capture_admission_distinguishes_prepared_context_and_closed_unavailable_reasons() {
    let fixture = PhaseFixture::new();
    let private = fixture.0.join("vault");
    let saves = fixture.0.join("saves");
    let observations = fixture.0.join("observations");
    let actual = [
        PhaseSidecar::prepare_captured(&private, None).1,
        PhaseSidecar::in_directory_captured(&observations, &private, &saves).1,
        PhaseSidecar::in_directory_captured(&private, &private, &saves).1,
        PhaseSidecar::in_directory_captured(&fixture.0.join("missing"), &private, &saves).1,
        PhaseSidecar::in_directory_captured(
            &observations,
            &fixture.0.join("missing-private"),
            &saves,
        )
        .1,
    ];
    assert_eq!(
        actual,
        [
            PhaseAdmission::Unavailable(PhaseAdmissionFailure::NoContext),
            PhaseAdmission::Prepared,
            PhaseAdmission::Unavailable(PhaseAdmissionFailure::InsideProtectedRoot),
            PhaseAdmission::Unavailable(PhaseAdmissionFailure::TempMetadataUnavailable),
            PhaseAdmission::Unavailable(PhaseAdmissionFailure::CanonicalizationUnavailable),
        ]
    );
}

#[test]
fn capture_terminal_read_distinguishes_empty_valid_invalid_and_oversize_without_raw_text() {
    let fixture = PhaseFixture::new();
    let mut sidecar = PhaseSidecar::in_directory(
        &fixture.0.join("observations"),
        &fixture.0.join("vault"),
        &fixture.0.join("saves"),
    )
    .unwrap();
    let cases: [&[u8]; 5] = [
        b"",
        b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SECURITY_MODULE_READY\nNFACL1|3|PRE_SET_ACL\n",
        b"NFACL1|1|SCRIPT_BODY_ENTERED",
        b"PRIVATE_PATH SID_SENTINEL EXCEPTION_SENTINEL\n",
        &[b'x'; 513],
    ];
    let mut actual = Vec::new();
    for bytes in cases {
        sidecar.file.set_len(0).unwrap();
        sidecar.file.seek(SeekFrom::Start(0)).unwrap();
        sidecar.file.write_all(bytes).unwrap();
        actual.push(sidecar.observe_captured_after_terminal(true));
    }
    assert_eq!(
        actual,
        [
            (ChildPhaseObservation::Unknown, PhaseRead::Empty),
            (
                ChildPhaseObservation::Observed {
                    last: HelperPhase::BeforeSetAcl,
                    records: 3,
                },
                PhaseRead::Observed,
            ),
            (ChildPhaseObservation::Unknown, PhaseRead::Invalid),
            (ChildPhaseObservation::Unknown, PhaseRead::Invalid),
            (
                ChildPhaseObservation::Unknown,
                PhaseRead::Failed(PhaseReadFailure::Oversize),
            ),
        ]
    );
    let rendered = format!("{actual:?}");
    for private in ["PRIVATE_PATH", "SID_SENTINEL", "EXCEPTION_SENTINEL"] {
        assert!(!rendered.contains(private));
    }
}

#[test]
fn capture_unconfirmed_terminal_preserves_position_and_reports_no_read() {
    let fixture = PhaseFixture::new();
    let mut sidecar = PhaseSidecar::in_directory(
        &fixture.0.join("observations"),
        &fixture.0.join("vault"),
        &fixture.0.join("saves"),
    )
    .unwrap();
    sidecar
        .file
        .write_all(b"NFACL1|1|SCRIPT_BODY_ENTERED\n")
        .unwrap();
    sidecar.file.seek(SeekFrom::Start(3)).unwrap();
    let actual = sidecar.observe_captured_after_terminal(false);
    assert_eq!(sidecar.file.stream_position().unwrap(), 3);
    assert_eq!(
        actual,
        (
            ChildPhaseObservation::Unknown,
            PhaseRead::TerminalUnconfirmed
        )
    );
}

#[test]
fn capture_bounded_read_payload_above_cap_is_oversize_not_generic_invalid() {
    // A bounded read can return513 after an earlier metadata check; this is a pure
    // payload control, not a claim that a real concurrent-file race was observed.
    assert_eq!(
        classify_bounded_read(&[b'x'; 513]),
        (
            ChildPhaseObservation::Unknown,
            PhaseRead::Failed(PhaseReadFailure::Oversize),
        )
    );
}
