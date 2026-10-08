use super::codec;
use crate::registration::{
    CampaignBinding, KnownRegistrationFrontier, RegisteredBranch, RegistrationError,
    RegistrationPolicy,
};
use nf_contract::identity::{DeviceId, RequestId};
use nf_identity::model::{IdentityError, Scope};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionMode {
    HeadlessLeaseOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionPolicy {
    pub mode: AdmissionMode,
    pub registration_policy_digest: [u8; 32],
}
impl AdmissionPolicy {
    pub fn digest(&self) -> [u8; 32] {
        crate::schema::hash(&codec::policy_bytes(self))
    }
    pub(super) fn validate(&self, registration: &RegistrationPolicy) -> Result<()> {
        registration.validate()?;
        if self.registration_policy_digest != registration.digest()? {
            return Err(LeaseError::Policy);
        }
        Ok(())
    }
}

/// An admission-local label, not a secret or a device credential.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ClientSessionId([u8; 16]);
impl ClientSessionId {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes == [0; 16] {
            return Err(LeaseError::InvalidSession);
        }
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Retained admission prefix; distinct from registration history and runtime sessions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownAdmissionFrontier {
    pub scope: Scope,
    pub revision: u64,
    pub head: [u8; 32],
    pub minimum_membership_revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmitLease {
    pub request: RequestId,
    pub binding: CampaignBinding,
    pub device: DeviceId,
    pub session: ClientSessionId,
    pub registration: RegisteredBranch,
    pub known_registration: KnownRegistrationFrontier,
    pub known_admission: KnownAdmissionFrontier,
    pub policy_digest: [u8; 32],
}

/// Historical durable receipt. Possession never authorizes an economic command.
/// Future commands must check current active session, generation, membership and taint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeaseGranted {
    pub mode: AdmissionMode,
    pub registration: RegisteredBranch,
    pub original_request: RequestId,
    pub device: DeviceId,
    pub session: ClientSessionId,
    pub generation: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeaseError {
    Identity(IdentityError),
    Registration(RegistrationError),
    Storage(crate::StoreError),
    Scope,
    Policy,
    InvalidSession,
    Limit,
    Replay,
    Expired,
    Entropy,
    Corrupt,
    StaleBackup,
    UnknownRegistration,
    Conflict,
    Unimplemented,
}
impl From<IdentityError> for LeaseError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}
impl From<RegistrationError> for LeaseError {
    fn from(error: RegistrationError) -> Self {
        Self::Registration(error)
    }
}
impl From<crate::StoreError> for LeaseError {
    fn from(error: crate::StoreError) -> Self {
        Self::Storage(error)
    }
}
impl From<rusqlite::Error> for LeaseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(crate::StoreError::from(error))
    }
}
pub(super) type Result<T> = std::result::Result<T, LeaseError>;
