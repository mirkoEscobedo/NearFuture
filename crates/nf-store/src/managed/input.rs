use super::{
    error::{ManagedError, Result},
    state::PendingPolicy,
};
use crate::migration::{Composition, ProviderVersion};
use crate::{KnownFrontiers, PrincipalDevice, Reservation};
use nf_kernel::{Frontier, JobResult, MissingPolicy};
use std::path::PathBuf;

/// Caller metadata only. Admission verifies the actual owned file separately.
#[derive(Clone, Debug)]
pub struct FilePin {
    pub(super) path: PathBuf,
    pub(super) bytes: u64,
    pub(super) digest: [u8; 32],
    pub(super) provenance: String,
}
impl FilePin {
    pub fn new(path: PathBuf, bytes: u64, digest: [u8; 32], provenance: &str) -> Result<Self> {
        let text = path.to_str().ok_or(ManagedError::Scope)?;
        if !path.is_absolute()
            || text.len() > 4096
            || text.as_bytes().contains(&0)
            || bytes == 0
            || bytes > 268_435_456
            || provenance.len() > 2048
            || provenance.as_bytes().contains(&0)
        {
            return Err(ManagedError::Limit);
        }
        Ok(Self {
            path,
            bytes,
            digest,
            provenance: provenance.to_string(),
        })
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
/// Pins support and custody data; this never certifies a running executable.
pub struct BinaryEvidence<'a> {
    pub(super) old: FilePin,
    pub(super) new: FilePin,
    pub(super) from: &'a Composition,
    pub(super) to: &'a Composition,
    pub(super) qualification: &'a str,
}
impl<'a> BinaryEvidence<'a> {
    pub fn library_fixture(
        old: FilePin,
        new: FilePin,
        from: &'a Composition,
        to: &'a Composition,
        qualification: &'a str,
    ) -> Self {
        Self {
            old,
            new,
            from,
            to,
            qualification,
        }
    }
}
pub struct AdoptionInput<'a> {
    pub source_path: PathBuf,
    pub minimum: KnownFrontiers,
    pub composition: &'a Composition,
    pub supported: &'a [ProviderVersion],
    pub binary_evidence: BinaryEvidence<'a>,
    pub artifact_provenance: &'a str,
}
pub struct PrepareInput<'a> {
    pub frontier: &'a Frontier,
    pub devices: &'a [PrincipalDevice],
    pub reservations: &'a [Reservation],
    pub principals: &'a [super::challenge::PrincipalProof],
    pub policy: PendingPolicy,
    pub admitted_at_ms: u64,
    pub cutoff_ms: u64,
}
pub struct SettlementInput {
    pub results: Vec<JobResult>,
    pub missing_policy: MissingPolicy,
}
