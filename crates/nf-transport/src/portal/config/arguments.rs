use super::{MAX_PATH_BYTES, PeerError, PortalMode, primitives::number};
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    time::Duration,
};
/// Bounded operator input only. These paths and minima grant no filesystem or policy authority.
#[derive(Clone, Debug)]
pub struct PortalArguments {
    pub mode: PortalMode,
    pub vault: PathBuf,
    pub save_root: PathBuf,
    pub database: PathBuf,
    pub config: PathBuf,
    pub slot: Option<u8>,
    pub duration: Option<Duration>,
}
impl PortalArguments {
    /// Arguments exclude the executable name. Refuses before any file or network effect.
    pub fn parse(args: &[OsString]) -> Result<Self, PeerError> {
        let mode = match args.first().and_then(|s| s.to_str()) {
            Some("serve") => PortalMode::Serve,
            Some("watch") => PortalMode::Watch,
            Some("init-book") => PortalMode::InitializeBook,
            _ => return Err(PeerError::Malformed),
        };
        let length = match mode {
            PortalMode::Serve => 6,
            PortalMode::Watch => 7,
            PortalMode::InitializeBook => 5,
        };
        if args.len() != length {
            return Err(PeerError::Malformed);
        }
        let mut parsed = Self {
            mode,
            vault: path(&args[1])?,
            save_root: path(&args[2])?,
            database: path(&args[3])?,
            config: path(&args[4])?,
            slot: None,
            duration: None,
        };
        if mode == PortalMode::Watch {
            parsed.slot = Some(number(args[5].to_str().ok_or(PeerError::Malformed)?, 7)? as u8);
        }
        if mode != PortalMode::InitializeBook {
            let milliseconds = number(
                args[length - 1].to_str().ok_or(PeerError::Malformed)?,
                60_000,
            )?;
            if milliseconds < 500 {
                return Err(PeerError::Limit);
            }
            parsed.duration = Some(Duration::from_millis(milliseconds));
        }
        Ok(parsed)
    }
}
pub(super) fn path(input: &OsStr) -> Result<PathBuf, PeerError> {
    let text = input.to_str().ok_or(PeerError::Malformed)?;
    if text.is_empty()
        || text.len() > MAX_PATH_BYTES
        || text.bytes().any(|b| matches!(b, 0 | b'\r' | b'\n'))
    {
        return Err(PeerError::Malformed);
    }
    Ok(PathBuf::from(input))
}
