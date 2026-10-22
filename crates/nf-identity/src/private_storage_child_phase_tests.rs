use super::*;

#[path = "private_storage_capture_tests.rs"]
mod capture;

#[path = "private_storage_writer_health_tests.rs"]
mod writer_health;

#[test]
fn child_phase_observation_unavailable_never_creates_inside_protected_roots() {
    let fixture = PhaseFixture::new();
    let private = fixture.0.join("vault");
    let saves = fixture.0.join("saves");
    assert!(PhaseSidecar::prepare(&private, None).is_none());
    for directory in [&private, &saves, &fixture.0.join("does-not-exist")] {
        assert!(PhaseSidecar::in_directory(directory, &private, &saves).is_none());
    }
    assert_eq!(std::fs::read_dir(&private).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&saves).unwrap().count(), 0);
}

#[test]
fn child_phase_owned_file_malformed_empty_and_oversize_are_unknown() {
    use std::io::{Seek, SeekFrom, Write};
    let fixture = PhaseFixture::new();
    for bytes in [
        b"".as_slice(),
        b"NFACL1|1|SCRIPT_BODY_ENTERED",
        &[b'x'; 513],
    ] {
        let mut sidecar = PhaseSidecar::in_directory(
            &fixture.0.join("observations"),
            &fixture.0.join("vault"),
            &fixture.0.join("saves"),
        )
        .unwrap();
        sidecar.file.write_all(bytes).unwrap();
        sidecar.file.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(
            sidecar.observe_after_terminal(true),
            ChildPhaseObservation::Unknown
        );
    }
}

#[test]
fn child_phase_accepts_initialize_verify_and_refusal_prefixes_but_not_helper_text() {
    assert_eq!(
        parse(b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SECURITY_MODULE_READY\nNFACL1|3|PRE_SET_ACL\nNFACL1|4|POST_SET_ACL\nNFACL1|5|PRE_GET_ACL\nNFACL1|6|POST_GET_ACL\nNFACL1|7|POLICY_VERIFIED\n"),
        ChildPhaseObservation::Observed { last: HelperPhase::PolicyVerified, records: 7 }
    );
    assert_eq!(
        parse(b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SECURITY_MODULE_READY\nNFACL1|3|PRE_GET_ACL\nNFACL1|4|POST_GET_ACL\nNFACL1|5|POLICY_VERIFIED\n"),
        ChildPhaseObservation::Observed { last: HelperPhase::PolicyVerified, records: 5 }
    );
    assert_eq!(
        parse(b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|REFUSED\n"),
        ChildPhaseObservation::Observed {
            last: HelperPhase::Refused,
            records: 2
        }
    );
    for bytes in [
        b"".as_slice(),
        b"NFACL1|1|SCRIPT_BODY_ENTERED",
        b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|3|SECURITY_MODULE_READY\n",
        b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SCRIPT_BODY_ENTERED\n",
        b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|POST_SET_ACL\n",
        b"NFACL1|1|REFUSED\n",
        b"NFACL1|01|SCRIPT_BODY_ENTERED\n",
        b"NFACL1|1|SCRIPT_BODY_ENTERED\r\n",
        b"NFACL1|1|SCRIPT_BODY_ENTERED|PRIVATE_PATH\n",
        b"NFACL1|1|SID_SENTINEL EXCEPTION_SENTINEL\n",
        &[0xff, b'\n'],
    ] {
        assert_eq!(parse(bytes), ChildPhaseObservation::Unknown);
        let rendered = format!("{:?}", parse(bytes));
        for private in ["PRIVATE_PATH", "SID_SENTINEL", "EXCEPTION_SENTINEL"] {
            assert!(!rendered.contains(private));
        }
    }
    assert_eq!(parse(&[b'x'; 513]), ChildPhaseObservation::Unknown);
}

struct PhaseFixture(std::path::PathBuf);
impl PhaseFixture {
    fn new() -> Self {
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let nonce = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let root = std::env::temp_dir().join(format!("nf-acl-phase-test-{nonce}"));
        std::fs::create_dir(&root).unwrap();
        for directory in ["vault", "saves", "observations"] {
            std::fs::create_dir(root.join(directory)).unwrap();
        }
        Self(root)
    }
}
impl Drop for PhaseFixture {
    fn drop(&mut self) {
        // Only the fresh random root created by this fixture is eligible.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn child_phase_sidecar_uses_fresh_outside_file_and_no_observation_before_owned_terminal() {
    use std::io::{Seek, SeekFrom, Write};
    let fixture = PhaseFixture::new();
    let mut sidecar = PhaseSidecar::in_directory(
        &fixture.0.join("observations"),
        &fixture.0.join("vault"),
        &fixture.0.join("saves"),
    )
    .expect("best-effort sidecar should be available in this ordinary owned fixture");
    let path = sidecar.path().to_owned();
    assert_eq!(
        path.parent().unwrap(),
        fixture.0.join("observations").canonicalize().unwrap()
    );
    assert!(path.is_file());
    let bytes = b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SECURITY_MODULE_READY\n";
    sidecar.file.write_all(bytes).unwrap();
    sidecar.file.seek(SeekFrom::Start(3)).unwrap();
    assert_eq!(
        sidecar.observe_after_terminal(false),
        ChildPhaseObservation::Unknown
    );
    assert_eq!(
        sidecar.file.stream_position().unwrap(),
        3,
        "uncertain child must not cause a diagnostic read or seek"
    );
    assert_eq!(
        sidecar.observe_after_terminal(true),
        ChildPhaseObservation::Observed {
            last: HelperPhase::SecurityModuleReady,
            records: 2
        }
    );
    drop(sidecar);
    assert!(
        !path.exists(),
        "parent-owned sidecar is cleaned after terminal consumption"
    );
}

#[test]
fn child_phase_reports_only_the_last_complete_ordered_progress_marker() {
    let bytes =
        b"NFACL1|1|SCRIPT_BODY_ENTERED\nNFACL1|2|SECURITY_MODULE_READY\nNFACL1|3|PRE_SET_ACL\n";
    assert_eq!(
        parse(bytes),
        ChildPhaseObservation::Observed {
            last: HelperPhase::BeforeSetAcl,
            records: 3,
        }
    );
}
