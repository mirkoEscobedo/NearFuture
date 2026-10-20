use std::io::ErrorKind;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperKind {
    SingleAcl,
    BatchAcl,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperStep {
    Metadata,
    Reparse,
    Spawn,
    Wait,
    InputPipe,
    OutputPipe,
    ErrorPipe,
    InputWrite,
    OutputRead,
    ErrorRead,
    InputJoin,
    OutputJoin,
    ErrorJoin,
    Exit,
    OutputPolicy,
}
/// Closed locations/refusals, not an inference about the originating OS error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperExitStage {
    Unclassified,
    ReparseRefusal,
    OwnerProtectionRefusal,
    AccessRulesRefusal,
    ImportSecurity,
    GetItem,
    CurrentUserSid,
    InitializeAcl,
    GetAcl,
    InspectOwnerProtection,
    InspectRules,
    BatchRead,
    BatchCount,
    BatchPath,
    BatchTrailing,
    BatchInvoke,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelperCause {
    Refused,
    Io {
        kind: ErrorKind,
        os_code: Option<i32>,
    },
    Timeout {
        elapsed_ms: u64,
    },
    ChildExit {
        code: Option<i32>,
        stage: Option<HelperExitStage>,
    },
    MissingPipe,
    ThreadPanic,
    UnexpectedOutput {
        stdout: bool,
        stderr: bool,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelperDiagnostic {
    kind: HelperKind,
    step: HelperStep,
    cause: HelperCause,
}
impl HelperDiagnostic {
    #[cfg(windows)]
    pub(crate) fn new(kind: HelperKind, step: HelperStep, cause: HelperCause) -> Self {
        Self { kind, step, cause }
    }
    #[cfg(windows)]
    pub(crate) fn io(kind: HelperKind, step: HelperStep, error: std::io::Error) -> Self {
        Self::new(
            kind,
            step,
            HelperCause::Io {
                kind: error.kind(),
                os_code: error.raw_os_error(),
            },
        )
    }
    pub fn kind(self) -> HelperKind {
        self.kind
    }
    pub fn step(self) -> HelperStep {
        self.step
    }
    pub fn cause(self) -> HelperCause {
        self.cause
    }
}

#[cfg(windows)]
pub(crate) fn exit_stage(kind: HelperKind, code: Option<i32>) -> Option<HelperExitStage> {
    use HelperExitStage::*;
    Some(match code? {
        2 => Unclassified,
        3 => ReparseRefusal,
        4 => OwnerProtectionRefusal,
        5 => AccessRulesRefusal,
        10 => ImportSecurity,
        11 => GetItem,
        12 => CurrentUserSid,
        13 => InitializeAcl,
        14 => GetAcl,
        15 => InspectOwnerProtection,
        16 => InspectRules,
        20 if kind == HelperKind::BatchAcl => BatchRead,
        21 if kind == HelperKind::BatchAcl => BatchCount,
        22 if kind == HelperKind::BatchAcl => BatchPath,
        23 if kind == HelperKind::BatchAcl => BatchTrailing,
        24 if kind == HelperKind::BatchAcl => BatchInvoke,
        _ => return None,
    })
}
