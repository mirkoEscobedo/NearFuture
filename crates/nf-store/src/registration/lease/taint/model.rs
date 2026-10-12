use super::codec;
use crate::registration::lease::{
    AdmissionPolicy, ClientSessionId, KnownAdmissionFrontier, LeaseError, LeaseGranted,
};
use crate::registration::{
    CampaignBinding, KnownRegistrationFrontier, RegistrationError, RegistrationPolicy,
};
use nf_contract::identity::{CampaignId, DeviceId, RequestId};
use nf_identity::model::{IdentityError, Scope};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaintMode {
    HeadlessTaintOnly,
}

/// Trusted policy ancestry label, not native ancestry certification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthorityLineageId([u8; 32]);
impl AuthorityLineageId {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        if bytes == [0; 32] {
            return Err(TaintError::Policy);
        }
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CampaignLineage {
    pub campaign: CampaignId,
    pub lineage: AuthorityLineageId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintPolicy {
    pub mode: TaintMode,
    pub registration_policy_digest: [u8; 32],
    pub admission_policy_digest: [u8; 32],
    pub expected_detector: [u8; 32],
    pub expected_detector_version: u64,
    pub expected_manifest: [u8; 32],
    pub campaigns: Vec<CampaignLineage>,
}
impl TaintPolicy {
    pub fn digest(&self) -> Result<[u8; 32]> {
        if self.campaigns.is_empty() || self.campaigns.len() > 64 {
            return Err(TaintError::Limit);
        }
        Ok(crate::schema::hash(&codec::policy_bytes(self)))
    }
    pub(super) fn validate(
        &self,
        registration: &RegistrationPolicy,
        admission: &AdmissionPolicy,
    ) -> Result<()> {
        admission.validate(registration)?;
        if self.campaigns.is_empty() || self.campaigns.len() > 64 {
            return Err(TaintError::Limit);
        }
        if self.registration_policy_digest != registration.digest()?
            || self.admission_policy_digest != admission.digest()
            || self.expected_detector == [0; 32]
            || self.expected_detector_version == 0
            || self.expected_manifest == [0; 32]
            || self
                .campaigns
                .windows(2)
                .any(|pair| pair[0].campaign >= pair[1].campaign)
        {
            return Err(TaintError::Policy);
        }
        let declared: BTreeSet<_> = registration
            .allowed
            .iter()
            .map(|entry| entry.campaign)
            .collect();
        let mapped: BTreeSet<_> = self.campaigns.iter().map(|entry| entry.campaign).collect();
        if mapped != declared {
            return Err(TaintError::Policy);
        }
        Ok(())
    }
    pub(super) fn lineage(&self, campaign: CampaignId) -> Result<AuthorityLineageId> {
        self.campaigns
            .binary_search_by_key(&campaign, |entry| entry.campaign)
            .map(|index| self.campaigns[index].lineage)
            .map_err(|_| TaintError::Policy)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownTaintFrontier {
    pub scope: Scope,
    pub revision: u64,
    pub head: [u8; 32],
    pub minimum_membership_revision: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkProhibitedManifest {
    pub request: RequestId,
    pub binding: CampaignBinding,
    pub device: DeviceId,
    pub session: ClientSessionId,
    pub grant: LeaseGranted,
    pub lineage: AuthorityLineageId,
    pub known_registration: KnownRegistrationFrontier,
    pub known_admission: KnownAdmissionFrontier,
    pub known_taint: KnownTaintFrontier,
    pub policy_digest: [u8; 32],
    pub expected_detector: [u8; 32],
    pub expected_detector_version: u64,
    pub expected_manifest: [u8; 32],
    pub observed_manifest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaintCause {
    SelfReportedProhibitedManifest,
}

/// Durable self-report receipt only; never native attestation or an economic capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaintRecorded {
    pub original_request: RequestId,
    pub binding: CampaignBinding,
    pub lineage: AuthorityLineageId,
    pub revision: u64,
    pub cause: TaintCause,
    pub expected_detector: [u8; 32],
    pub expected_detector_version: u64,
    pub expected_manifest: [u8; 32],
    pub observed_manifest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaintError {
    Identity(IdentityError),
    Registration(RegistrationError),
    Lease(LeaseError),
    Storage(crate::StoreError),
    Scope,
    Policy,
    Limit,
    Replay,
    Expired,
    Entropy,
    Corrupt,
    StaleBackup,
    UnsupportedProfile,
    UnknownGrant,
    Conflict,
    Unimplemented,
}
impl From<IdentityError> for TaintError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}
impl From<RegistrationError> for TaintError {
    fn from(error: RegistrationError) -> Self {
        Self::Registration(error)
    }
}
impl From<LeaseError> for TaintError {
    fn from(error: LeaseError) -> Self {
        Self::Lease(error)
    }
}
impl From<crate::StoreError> for TaintError {
    fn from(error: crate::StoreError) -> Self {
        Self::Storage(error)
    }
}
impl From<rusqlite::Error> for TaintError {
    fn from(error: rusqlite::Error) -> Self {
        crate::StoreError::from(error).into()
    }
}
pub(super) type Result<T> = std::result::Result<T, TaintError>;
