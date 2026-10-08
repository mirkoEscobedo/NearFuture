use nf_contract::identity::{AccountId, BranchId, CampaignId, DeviceId, RequestId};
use nf_identity::model::{IdentityError, Scope};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationMode {
    HeadlessRegistrationOnly,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AllowedBranch {
    pub campaign: CampaignId,
    pub branch: BranchId,
    pub account: AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrationPolicy {
    pub scope: Scope,
    pub mode: RegistrationMode,
    pub allowed: Vec<AllowedBranch>,
}

impl RegistrationPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.allowed.is_empty() || self.allowed.len() > 64 {
            return Err(RegistrationError::Limit);
        }
        if self.allowed.windows(2).any(|pair| {
            pair[0] >= pair[1]
                || (pair[0].campaign, pair[0].branch) == (pair[1].campaign, pair[1].branch)
        }) {
            return Err(RegistrationError::Policy);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<[u8; 32]> {
        Ok(crate::schema::hash(&super::codec::policy_bytes(self)?))
    }

    pub(super) fn permit(&self, request: &RegisterBranch) -> Result<()> {
        if request.scope != self.scope {
            return Err(RegistrationError::Scope);
        }
        let binding = AllowedBranch {
            campaign: request.campaign,
            branch: request.branch,
            account: request.account,
        };
        if request.policy_digest != self.digest()? || self.allowed.binary_search(&binding).is_err()
        {
            return Err(RegistrationError::Policy);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CampaignBinding {
    pub scope: Scope,
    pub campaign: CampaignId,
    pub branch: BranchId,
    pub account: AccountId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegisterBranch {
    pub request: RequestId,
    pub scope: Scope,
    pub campaign: CampaignId,
    pub branch: BranchId,
    pub account: AccountId,
    pub device: DeviceId,
    pub policy_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegisteredBranch {
    pub binding: CampaignBinding,
    pub original_request: RequestId,
    pub revision: u64,
}

/// Retain independently of restored storage. `head` identifies the prefix at `revision`,
/// not the latest head when legitimate later registrations have advanced the journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownRegistrationFrontier {
    pub scope: Scope,
    pub revision: u64,
    pub head: [u8; 32],
    pub minimum_membership_revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationError {
    Identity(IdentityError),
    Storage(crate::StoreError),
    Policy,
    Scope,
    Limit,
    Replay,
    Expired,
    Entropy,
    Corrupt,
    UnsupportedProfile,
    StaleBackup,
    Conflict,
    Unimplemented,
}

impl From<IdentityError> for RegistrationError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}
impl From<crate::StoreError> for RegistrationError {
    fn from(error: crate::StoreError) -> Self {
        Self::Storage(error)
    }
}
impl From<rusqlite::Error> for RegistrationError {
    fn from(error: rusqlite::Error) -> Self {
        crate::StoreError::from(error).into()
    }
}
pub(super) type Result<T> = std::result::Result<T, RegistrationError>;
