use super::HelperDiagnostic;
use crate::model::IdentityError;
use std::io::ErrorKind;

/// The existing error plus passive evidence from this call only. No path or private bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrivateFailure {
    error: IdentityError,
    diagnostic: Option<PrivateDiagnostic>,
}
impl PrivateFailure {
    pub fn identity_error(self) -> IdentityError {
        self.error
    }
    pub fn diagnostic(self) -> Option<PrivateDiagnostic> {
        self.diagnostic
    }
    pub(crate) fn io(
        operation: PrivateOperation,
        stage: PrivateStage,
        error: std::io::Error,
    ) -> Self {
        Self {
            error: IdentityError::PrivateStorage,
            diagnostic: Some(PrivateDiagnostic {
                operation,
                stage,
                cause: PrivateCause::Io {
                    kind: error.kind(),
                    os_code: error.raw_os_error(),
                },
            }),
        }
    }
    pub(crate) fn with_cause(
        operation: PrivateOperation,
        stage: PrivateStage,
        cause: PrivateCause,
    ) -> Self {
        Self {
            error: IdentityError::PrivateStorage,
            diagnostic: Some(PrivateDiagnostic {
                operation,
                stage,
                cause,
            }),
        }
    }
    pub(crate) fn legacy(error: IdentityError) -> Self {
        Self {
            error,
            diagnostic: None,
        }
    }
}
pub type PrivateResult<T> = Result<T, PrivateFailure>;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateOperation {
    IdentityCreate,
    BlobCreate,
    BlobScan,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateStage {
    Name,
    RootAccess,
    CreateNew,
    EntryAccess,
    Write,
    Sync,
    DirectoryOpen,
    DirectorySync,
    Encoding,
    RootMetadata,
    EntryMetadata,
    EntryType,
    ProvisionalAccess,
    PayloadOpen,
    PayloadMetadata,
    PayloadRead,
    FinalAccess,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivateCause {
    Refused,
    Io {
        kind: ErrorKind,
        os_code: Option<i32>,
    },
    Helper(HelperDiagnostic),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrivateDiagnostic {
    operation: PrivateOperation,
    stage: PrivateStage,
    cause: PrivateCause,
}
impl PrivateDiagnostic {
    pub fn operation(self) -> PrivateOperation {
        self.operation
    }
    pub fn stage(self) -> PrivateStage {
        self.stage
    }
    pub fn cause(self) -> PrivateCause {
        self.cause
    }
}
