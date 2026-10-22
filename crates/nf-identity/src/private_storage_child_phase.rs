//! Bounded, closed helper progress; never helper text or storage authority.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A complete published progress marker, not proof of the operation active at termination.
pub enum HelperPhase {
    ScriptBodyEntered,
    SecurityModuleReady,
    BeforeSetAcl,
    AfterSetAcl,
    BeforeGetAcl,
    AfterGetAcl,
    PolicyVerified,
    Refused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Best-effort closed evidence. Missing, invalid or concurrently owned streams stay unknown.
pub enum ChildPhaseObservation {
    Unknown,
    Observed { last: HelperPhase, records: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseAdmissionFailure {
    NoContext,
    TempMetadataUnavailable,
    UnsafeTempAncestry,
    CanonicalizationUnavailable,
    InsideProtectedRoot,
    NonceUnavailable,
    CreateUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseAdmission {
    Prepared,
    Unavailable(PhaseAdmissionFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseReadFailure {
    MetadataUnavailable,
    NonRegular,
    Oversize,
    SeekUnavailable,
    ReadUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseRead {
    NotAttempted,
    TerminalUnconfirmed,
    Failed(PhaseReadFailure),
    Empty,
    Invalid,
    Observed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Capture availability only; no new storage authority or helper execution inference.
pub struct PhaseCaptureObservation {
    pub admission: PhaseAdmission,
    pub read: PhaseRead,
}

#[cfg(any(windows, test))]
fn classify_bounded_read(bytes: &[u8]) -> (ChildPhaseObservation, PhaseRead) {
    if bytes.len() > 512 {
        return (
            ChildPhaseObservation::Unknown,
            PhaseRead::Failed(PhaseReadFailure::Oversize),
        );
    }
    if bytes.is_empty() {
        return (ChildPhaseObservation::Unknown, PhaseRead::Empty);
    }
    let phase = parse(bytes);
    let read = match phase {
        ChildPhaseObservation::Unknown => PhaseRead::Invalid,
        ChildPhaseObservation::Observed { .. } => PhaseRead::Observed,
    };
    (phase, read)
}

#[cfg(any(windows, test))]
pub(super) fn parse(bytes: &[u8]) -> ChildPhaseObservation {
    if bytes.is_empty() || bytes.len() > 512 || !bytes.ends_with(b"\n") {
        return ChildPhaseObservation::Unknown;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return ChildPhaseObservation::Unknown;
    };
    let mut last = None;
    let mut records = 0_u8;
    for line in text.split_terminator('\n') {
        records += 1;
        if records > 8 {
            return ChildPhaseObservation::Unknown;
        }
        let mut parts = line.split('|');
        if parts.next() != Some("NFACL1") || parts.next() != Some(records.to_string().as_str()) {
            return ChildPhaseObservation::Unknown;
        }
        let next = match parts.next() {
            Some("SCRIPT_BODY_ENTERED") => HelperPhase::ScriptBodyEntered,
            Some("SECURITY_MODULE_READY") => HelperPhase::SecurityModuleReady,
            Some("PRE_SET_ACL") => HelperPhase::BeforeSetAcl,
            Some("POST_SET_ACL") => HelperPhase::AfterSetAcl,
            Some("PRE_GET_ACL") => HelperPhase::BeforeGetAcl,
            Some("POST_GET_ACL") => HelperPhase::AfterGetAcl,
            Some("POLICY_VERIFIED") => HelperPhase::PolicyVerified,
            Some("REFUSED") => HelperPhase::Refused,
            _ => return ChildPhaseObservation::Unknown,
        };
        let allowed = matches!(
            (last, next),
            (None, HelperPhase::ScriptBodyEntered)
                | (
                    Some(HelperPhase::ScriptBodyEntered),
                    HelperPhase::SecurityModuleReady
                )
                | (
                    Some(HelperPhase::SecurityModuleReady),
                    HelperPhase::BeforeSetAcl | HelperPhase::BeforeGetAcl
                )
                | (Some(HelperPhase::BeforeSetAcl), HelperPhase::AfterSetAcl)
                | (Some(HelperPhase::AfterSetAcl), HelperPhase::BeforeGetAcl)
                | (Some(HelperPhase::BeforeGetAcl), HelperPhase::AfterGetAcl)
                | (Some(HelperPhase::AfterGetAcl), HelperPhase::PolicyVerified)
                | (
                    Some(
                        HelperPhase::ScriptBodyEntered
                            | HelperPhase::SecurityModuleReady
                            | HelperPhase::BeforeSetAcl
                            | HelperPhase::AfterSetAcl
                            | HelperPhase::BeforeGetAcl
                            | HelperPhase::AfterGetAcl
                    ),
                    HelperPhase::Refused
                )
        );
        if !allowed || parts.next().is_some() {
            return ChildPhaseObservation::Unknown;
        }
        last = Some(next);
    }
    match last {
        Some(last) => ChildPhaseObservation::Observed { last, records },
        None => ChildPhaseObservation::Unknown,
    }
}

#[cfg(any(windows, test))]
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[cfg(any(windows, test))]
pub(super) struct PhaseSidecar {
    file: File,
    path: PathBuf,
}

#[cfg(any(windows, test))]
impl PhaseSidecar {
    pub(super) fn prepare_captured(
        private_path: &Path,
        save_root: Option<&Path>,
    ) -> (Option<Self>, PhaseAdmission) {
        let Some(save_root) = save_root else {
            return (
                None,
                PhaseAdmission::Unavailable(PhaseAdmissionFailure::NoContext),
            );
        };
        Self::in_directory_captured(&std::env::temp_dir(), private_path, save_root)
    }

    fn in_directory_captured(
        directory: &Path,
        private_path: &Path,
        save_root: &Path,
    ) -> (Option<Self>, PhaseAdmission) {
        match Self::try_in_directory(directory, private_path, save_root) {
            Ok(sidecar) => (Some(sidecar), PhaseAdmission::Prepared),
            Err(reason) => (None, PhaseAdmission::Unavailable(reason)),
        }
    }

    pub(super) fn observe_captured_after_terminal(
        &mut self,
        confirmed: bool,
    ) -> (ChildPhaseObservation, PhaseRead) {
        if !confirmed {
            return (
                ChildPhaseObservation::Unknown,
                PhaseRead::TerminalUnconfirmed,
            );
        }
        self.read_bounded()
            .unwrap_or_else(|reason| (ChildPhaseObservation::Unknown, PhaseRead::Failed(reason)))
    }

    #[cfg(test)]
    pub(super) fn prepare(private_path: &Path, save_root: Option<&Path>) -> Option<Self> {
        Self::prepare_captured(private_path, save_root).0
    }

    #[cfg(test)]
    fn in_directory(directory: &Path, private_path: &Path, save_root: &Path) -> Option<Self> {
        Self::in_directory_captured(directory, private_path, save_root).0
    }

    fn try_in_directory(
        directory: &Path,
        private_path: &Path,
        save_root: &Path,
    ) -> Result<Self, PhaseAdmissionFailure> {
        for ancestor in directory.ancestors() {
            let metadata = fs::symlink_metadata(ancestor)
                .map_err(|_| PhaseAdmissionFailure::TempMetadataUnavailable)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(PhaseAdmissionFailure::UnsafeTempAncestry);
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(PhaseAdmissionFailure::UnsafeTempAncestry);
                }
            }
        }
        let directory = directory
            .canonicalize()
            .map_err(|_| PhaseAdmissionFailure::CanonicalizationUnavailable)?;
        if directory.starts_with(
            private_path
                .canonicalize()
                .map_err(|_| PhaseAdmissionFailure::CanonicalizationUnavailable)?,
        ) || directory.starts_with(
            save_root
                .canonicalize()
                .map_err(|_| PhaseAdmissionFailure::CanonicalizationUnavailable)?,
        ) {
            return Err(PhaseAdmissionFailure::InsideProtectedRoot);
        }
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| PhaseAdmissionFailure::NonceUnavailable)?;
        let nonce = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = directory.join(format!(".nf-acl-phase-{nonce}.tmp"));
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(&path)
            .map_err(|_| PhaseAdmissionFailure::CreateUnavailable)?;
        Ok(Self { file, path })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Never inspect a stream while the owned child's terminal state is uncertain.
    #[cfg(test)]
    pub(super) fn observe_after_terminal(&mut self, confirmed: bool) -> ChildPhaseObservation {
        self.observe_captured_after_terminal(confirmed).0
    }

    fn read_bounded(&mut self) -> Result<(ChildPhaseObservation, PhaseRead), PhaseReadFailure> {
        let metadata = self
            .file
            .metadata()
            .map_err(|_| PhaseReadFailure::MetadataUnavailable)?;
        if !metadata.is_file() {
            return Err(PhaseReadFailure::NonRegular);
        }
        if metadata.len() > 512 {
            return Err(PhaseReadFailure::Oversize);
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| PhaseReadFailure::SeekUnavailable)?;
        let mut bytes = Vec::new();
        (&mut self.file)
            .take(513)
            .read_to_end(&mut bytes)
            .map_err(|_| PhaseReadFailure::ReadUnavailable)?;
        Ok(classify_bounded_read(&bytes))
    }
}

#[cfg(any(windows, test))]
impl Drop for PhaseSidecar {
    fn drop(&mut self) {
        // The only deletion target is this parent's create-new random file.
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
#[path = "private_storage_child_phase_tests.rs"]
mod tests;
