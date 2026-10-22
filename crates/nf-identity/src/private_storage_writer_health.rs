//! Independent, bounded evidence about the first primary phase publication.
#[cfg(any(windows, test))]
use super::child_phase::{PhaseAdmission, PhaseReadFailure, PhaseSidecar};
use super::child_phase::{PhaseAdmissionFailure, PhaseRead};
#[cfg(any(windows, test))]
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirstWriterHealth {
    Unknown,
    BodyEntered,
    FirstPublished,
    /// An attempted publication did not fully complete; partial bytes may exist.
    FirstFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthAdmission {
    NotAttempted,
    Prepared,
    Unavailable(PhaseAdmissionFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HelperHealthObservation {
    pub observation: FirstWriterHealth,
    pub admission: HealthAdmission,
    pub read: PhaseRead,
}

#[cfg(any(windows, test))]
impl HelperHealthObservation {
    pub(super) fn not_attempted(admission: HealthAdmission) -> Self {
        Self {
            observation: FirstWriterHealth::Unknown,
            admission,
            read: PhaseRead::NotAttempted,
        }
    }
}

#[cfg(any(windows, test))]
pub(super) fn prepare(
    private_path: &Path,
    save_root: Option<&Path>,
    primary_prepared: bool,
) -> (Option<PhaseSidecar>, HealthAdmission) {
    if !primary_prepared {
        return (None, HealthAdmission::NotAttempted);
    }
    let (sidecar, admission) = PhaseSidecar::prepare_captured(private_path, save_root);
    let admission = match admission {
        PhaseAdmission::Prepared => HealthAdmission::Prepared,
        PhaseAdmission::Unavailable(reason) => HealthAdmission::Unavailable(reason),
    };
    (sidecar, admission)
}

#[cfg(any(windows, test))]
pub(super) fn classify(bytes: &[u8]) -> (FirstWriterHealth, PhaseRead) {
    if bytes.len() > 64 {
        return (
            FirstWriterHealth::Unknown,
            PhaseRead::Failed(PhaseReadFailure::Oversize),
        );
    }
    if bytes.is_empty() {
        return (FirstWriterHealth::Unknown, PhaseRead::Empty);
    }
    let observation = match bytes {
        b"NFACLH1|1|BODY\n" => FirstWriterHealth::BodyEntered,
        b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_PUBLISHED\n" => FirstWriterHealth::FirstPublished,
        b"NFACLH1|1|BODY\nNFACLH1|2|FIRST_FAILED\n" => FirstWriterHealth::FirstFailed,
        _ => return (FirstWriterHealth::Unknown, PhaseRead::Invalid),
    };
    (observation, PhaseRead::Observed)
}

#[cfg(any(windows, test))]
pub(super) fn observe(
    sidecar: Option<&mut PhaseSidecar>,
    admission: HealthAdmission,
    terminal_confirmed: bool,
) -> HelperHealthObservation {
    let Some(sidecar) = sidecar else {
        return HelperHealthObservation::not_attempted(admission);
    };
    let (observation, read) = if !terminal_confirmed {
        (FirstWriterHealth::Unknown, PhaseRead::TerminalUnconfirmed)
    } else {
        match sidecar.read_bounded_bytes(64) {
            Ok(bytes) => classify(&bytes),
            Err(reason) => (FirstWriterHealth::Unknown, PhaseRead::Failed(reason)),
        }
    };
    HelperHealthObservation {
        observation,
        admission,
        read,
    }
}
