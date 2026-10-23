//! Closed storage diagnostics: no paths, helper text, identity or private payloads.
use super::child_phase::{ChildPhaseObservation, PhaseCaptureObservation};
use super::writer_health::HelperHealthObservation;
use crate::model::IdentityError;
use std::io;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateStorageStage {
    CreateDirectory,
    InitializeAcl,
    VerifyAcl,
    CanonicalizeVault,
    CanonicalizeSaves,
    Placement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateStorageReason {
    Io,
    Metadata,
    Reparse,
    UnsafeType,
    Spawn,
    Wait,
    Deadline,
    Exit,
    InsideSaves,
    DirectoryCheck,
    Permissions,
    UnsupportedPlatform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageIoFailure {
    pub kind: io::ErrorKind,
    pub os_code: Option<i32>,
}
impl From<&io::Error> for StorageIoFailure {
    fn from(error: &io::Error) -> Self {
        Self {
            kind: error.kind(),
            os_code: error.raw_os_error(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelperCleanup {
    pub kill_error: Option<StorageIoFailure>,
    pub wait_result: Result<Option<i32>, StorageIoFailure>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelperObservation {
    pub spawn_micros: u128,
    pub poll_micros: Option<u128>,
    pub exit_code: Option<i32>,
    pub cleanup: Option<HelperCleanup>,
    /// Last complete marker after confirmed terminal; never helper text or authority values.
    pub child_phase: ChildPhaseObservation,
    /// Availability of this same bounded capture, without raw helper or authority values.
    pub phase_capture: PhaseCaptureObservation,
    /// Independent first-publication evidence after confirmed terminal, never raw helper output.
    pub writer_health: HelperHealthObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrivateStorageFailure {
    pub stage: PrivateStorageStage,
    pub reason: PrivateStorageReason,
    pub io: Option<StorageIoFailure>,
    pub helper: Option<HelperObservation>,
}
impl PrivateStorageFailure {
    pub(super) fn new(stage: PrivateStorageStage, reason: PrivateStorageReason) -> Self {
        Self {
            stage,
            reason,
            io: None,
            helper: None,
        }
    }
    pub(super) fn with_io(
        stage: PrivateStorageStage,
        reason: PrivateStorageReason,
        error: &io::Error,
    ) -> Self {
        Self {
            io: Some(error.into()),
            ..Self::new(stage, reason)
        }
    }
    pub(super) fn legacy(self) -> IdentityError {
        IdentityError::PrivateStorage
    }
}

#[cfg(test)]
#[path = "private_storage_diagnostic_tests.rs"]
mod tests;
