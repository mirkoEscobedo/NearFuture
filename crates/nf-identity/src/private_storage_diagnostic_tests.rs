use super::*;

#[test]
fn child_phase_failure_retains_only_closed_progress_and_legacy_error() {
    use super::super::child_phase::{ChildPhaseObservation, HelperPhase};
    let source = io::Error::other("PRIVATE_PATH_SENTINEL SID_SENTINEL EXCEPTION_SENTINEL");
    let mut failure = PrivateStorageFailure::with_io(
        PrivateStorageStage::InitializeAcl,
        PrivateStorageReason::Deadline,
        &source,
    );
    failure.helper = Some(HelperObservation {
        spawn_micros: 1,
        poll_micros: Some(5_000_000),
        exit_code: None,
        cleanup: None,
        child_phase: ChildPhaseObservation::Observed {
            last: HelperPhase::BeforeGetAcl,
            records: 5,
        },
    });
    assert_eq!(
        failure.helper.unwrap().child_phase,
        ChildPhaseObservation::Observed {
            last: HelperPhase::BeforeGetAcl,
            records: 5,
        }
    );
    let rendered = format!("{failure:?}");
    for private in [
        "PRIVATE_PATH_SENTINEL",
        "SID_SENTINEL",
        "EXCEPTION_SENTINEL",
    ] {
        assert!(!rendered.contains(private));
    }
    assert_eq!(failure.legacy(), IdentityError::PrivateStorage);
}

#[test]
fn io_diagnostics_drop_private_error_text_before_rendering() {
    let source =
        io::Error::other("PRIVATE_PATH_SENTINEL SID_SENTINEL ACL_SENTINEL PAYLOAD_SENTINEL");
    let failure = PrivateStorageFailure::with_io(
        PrivateStorageStage::InitializeAcl,
        PrivateStorageReason::Spawn,
        &source,
    );
    assert_eq!(failure.io.unwrap().kind, io::ErrorKind::Other);
    assert_eq!(failure.io.unwrap().os_code, None);
    let rendered = format!("{failure:?}");
    for secret in [
        "PRIVATE_PATH_SENTINEL",
        "SID_SENTINEL",
        "ACL_SENTINEL",
        "PAYLOAD_SENTINEL",
    ] {
        assert!(!rendered.contains(secret));
    }
    assert_eq!(failure.legacy(), IdentityError::PrivateStorage);
}
