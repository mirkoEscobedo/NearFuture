use nf_transport::{
    PeerError,
    portal_config::{PortalArguments, PortalMode},
};
use std::{ffi::OsString, time::Duration};
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(|value| OsString::from(*value)).collect()
}
#[test]
fn exact_command_shapes_keep_paths_slot_and_finite_duration_as_data() {
    let server =
        PortalArguments::parse(&args(&["serve", "vault", "save", "db", "cfg", "500"])).unwrap();
    assert_eq!(server.mode, PortalMode::Serve);
    assert_eq!(server.duration, Some(Duration::from_millis(500)));
    assert_eq!(server.slot, None);
    let watcher = PortalArguments::parse(&args(&[
        "watch", "vault", "save", "db", "cfg", "7", "60000",
    ]))
    .unwrap();
    assert_eq!(watcher.slot, Some(7));
    assert_eq!(watcher.duration, Some(Duration::from_secs(60)));
    let offline =
        PortalArguments::parse(&args(&["init-book", "vault", "save", "db", "cfg"])).unwrap();
    assert_eq!(offline.mode, PortalMode::InitializeBook);
    assert_eq!(offline.duration, None);
}
#[test]
fn malformed_arguments_refuse_without_opening_named_state() {
    for shape in [
        &[][..],
        &["serve"][..],
        &["watch", "v", "s", "d", "c", "0"][..],
        &["init-book", "v", "s", "d", "c", "500"][..],
        &["serve", "v", "s", "d", "c", "500", "--extra"][..],
        &["unknown", "v", "s", "d", "c"][..],
    ] {
        assert!(PortalArguments::parse(&args(shape)).is_err());
    }
    for time in ["499", "60001", "0500", "+500", "18446744073709551616"] {
        assert!(PortalArguments::parse(&args(&["serve", "v", "s", "d", "c", time])).is_err());
    }
    for slot in ["8", "00", "-1"] {
        assert!(
            PortalArguments::parse(&args(&["watch", "v", "s", "d", "c", slot, "500"])).is_err()
        );
    }
    for path in ["", "invalid\0path", "invalid\rpath", "invalid\npath"] {
        assert!(PortalArguments::parse(&args(&["serve", path, "s", "d", "c", "500"])).is_err());
    }
    let maximum = "x".repeat(4096);
    assert!(PortalArguments::parse(&args(&["serve", &maximum, "s", "d", "c", "500"])).is_ok());
    let oversized = "x".repeat(4097);
    assert_eq!(
        PortalArguments::parse(&args(&["serve", &oversized, "s", "d", "c", "500"])).unwrap_err(),
        PeerError::Malformed
    );
    let unicode = "é".repeat(2048);
    assert!(PortalArguments::parse(&args(&["serve", &unicode, "s", "d", "c", "500"])).is_ok());
    let oversized_utf8 = "é".repeat(2049);
    assert!(
        PortalArguments::parse(&args(&["serve", &oversized_utf8, "s", "d", "c", "500"])).is_err()
    );
}
#[cfg(unix)]
#[test]
fn non_utf8_host_argument_refuses() {
    use std::os::unix::ffi::OsStringExt;
    let mut input = args(&["serve", "v", "s", "d", "c", "500"]);
    input[1] = OsString::from_vec(vec![0xff]);
    assert!(PortalArguments::parse(&input).is_err());
}
#[cfg(windows)]
#[test]
fn unpaired_surrogate_host_argument_refuses() {
    use std::os::windows::ffi::OsStringExt;
    let mut input = args(&["serve", "v", "s", "d", "c", "500"]);
    input[1] = OsString::from_wide(&[0xd800]);
    assert!(PortalArguments::parse(&input).is_err());
}
