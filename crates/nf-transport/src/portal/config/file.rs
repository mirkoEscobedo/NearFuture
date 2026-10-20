use super::{MAX_CONFIG_BYTES, PeerError, PortalConfig, PortalMode};
use std::{
    fs::{File, Metadata},
    io::Read,
    path::Path,
};
/// One bounded opened regular file under the cooperative operator filesystem lifecycle.
/// Path preflight is not an atomic defence against concurrent owner alias replacement.
pub fn read_config(path: &Path, expected: PortalMode) -> Result<PortalConfig, PeerError> {
    super::arguments::path(path.as_os_str())?;
    regular(&std::fs::symlink_metadata(path).map_err(|_| PeerError::Storage)?)?;
    let mut file = File::open(path).map_err(|_| PeerError::Storage)?;
    let before = file.metadata().map_err(|_| PeerError::Storage)?;
    regular(&before)?;
    if before.len() > MAX_CONFIG_BYTES as u64 {
        return Err(PeerError::Limit);
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take(MAX_CONFIG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PeerError::Storage)?;
    let after = file.metadata().map_err(|_| PeerError::Storage)?;
    regular(&after)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(PeerError::Limit);
    }
    if before.len() != after.len() || after.len() != bytes.len() as u64 {
        return Err(PeerError::Storage);
    }
    PortalConfig::parse(&bytes, expected)
}
fn regular(metadata: &Metadata) -> Result<(), PeerError> {
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PeerError::Storage);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PeerError::Storage);
        }
    }
    Ok(())
}
