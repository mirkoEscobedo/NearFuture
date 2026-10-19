use super::*;

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
