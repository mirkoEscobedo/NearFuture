//! Exact safe std observations; not a full ACL or platform permission certificate.
use std::{fs::Metadata, path::Path};
#[derive(Clone, Eq, PartialEq)]
pub struct Permissions {
    pub readonly: bool,
    #[cfg(unix)]
    pub mode: u32,
    #[cfg(unix)]
    pub uid: u32,
    #[cfg(unix)]
    pub gid: u32,
    #[cfg(windows)]
    pub file_attributes: u32,
}
pub fn plain(metadata: &Metadata) {
    assert!(!metadata.file_type().is_symlink(), "symlink fixture refused");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(metadata.file_attributes() & 0x400, 0, "reparse fixture refused");
    }
    assert!(metadata.is_dir() || metadata.is_file(), "special fixture refused");
}
pub fn observe(metadata: &Metadata) -> Permissions {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;
    Permissions {
        readonly: metadata.permissions().readonly(),
        #[cfg(unix)]
        mode: metadata.mode(),
        #[cfg(unix)]
        uid: metadata.uid(),
        #[cfg(unix)]
        gid: metadata.gid(),
        #[cfg(windows)]
        file_attributes: metadata.file_attributes(),
    }
}
pub fn directory(path: &Path) {
    let metadata = std::fs::symlink_metadata(path).unwrap();
    plain(&metadata);
    assert!(metadata.is_dir(), "caller must create a plain directory");
}
pub const OBSERVATION_STRENGTH: &str =
    "Compared exact selected names/types/file bytes/readonly flags; Unix mode/uid/gid; Windows file_attributes. Full Windows DACL and extended POSIX ACL preservation UNVERIFIED. No permission mutator during product calls, no helper/child/FFI/new dependency. This is not full issue qualification.";
