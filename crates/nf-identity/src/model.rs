use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scope {
    pub universe: UniverseId,
    pub history: HistoryId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Roles(u8);
impl Roles {
    pub const PLAYER: Self = Self(1);
    pub const MODERATOR: Self = Self(2);
    pub const AUTHORITY_CANDIDATE: Self = Self(4);
    pub const REPLICA: Self = Self(8);
    pub const ARCHIVE: Self = Self(16);
    pub const RELAY: Self = Self(32);
    pub const WORKER: Self = Self(64);
    pub const ALL: Self = Self(127);
    pub fn from_bits(bits: u8) -> Result<Self, IdentityError> {
        if bits == 0 || bits & !127 != 0 {
            return Err(IdentityError::RolePolicy);
        }
        Ok(Self(bits))
    }
    pub fn bits(self) -> u8 {
        self.0
    }
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityError {
    Signature,
    Scope,
    Expired,
    RolePolicy,
    Frontier,
    UnknownAccount,
    UnknownDevice,
    Revoked,
    Replay,
    Conflict,
    Limit,
    Malformed,
    PrivateStorage,
    MissingLocalState,
    Persistence,
    Entropy,
    LostKeyRecoveryRefused,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicIdentity {
    pub account: AccountId,
    pub account_key: [u8; 32],
    pub device: DeviceId,
    pub device_key: [u8; 32],
    pub peer: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Account {
    pub key: [u8; 32],
    pub roles: Roles,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Device {
    pub account: AccountId,
    pub key: [u8; 32],
    pub peer: Vec<u8>,
    pub revoked: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipState {
    pub scope: Scope,
    pub revision: u64,
    pub owner: AccountId,
    pub accounts: BTreeMap<AccountId, Account>,
    pub devices: BTreeMap<DeviceId, Device>,
    pub consumed: BTreeSet<[u8; 16]>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invitation {
    pub scope: Scope,
    pub id: [u8; 16],
    pub issuer: AccountId,
    pub recipient: PublicIdentity,
    pub roles: Roles,
    pub expires_at: u64,
    pub issued_revision: u64,
    pub reusable: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedInvitation {
    pub invitation: Invitation,
    pub signature: [u8; 64],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionProof {
    pub account_signature: [u8; 64],
    pub device_signature: [u8; 64],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceProof {
    pub scope: Scope,
    pub account: AccountId,
    pub device: DeviceId,
    pub frontier: u64,
    pub peer: Vec<u8>,
    pub challenge: [u8; 32],
    pub signature: [u8; 64],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtectedOperation {
    Economic,
    Administration,
    Chat,
    Worker,
}

/// Atomic public state compare-and-swap. Private seeds never enter this port.
pub trait MembershipRepository {
    fn load_membership(&mut self, scope: Scope) -> Result<Option<MembershipState>, IdentityError>;
    fn commit_membership(
        &mut self,
        expected_revision: Option<u64>,
        next: &MembershipState,
    ) -> Result<(), IdentityError>;
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceRotation {
    pub scope: Scope,
    pub account: AccountId,
    pub device: DeviceId,
    pub frontier: u64,
    pub new_key: [u8; 32],
    pub new_peer: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountRotation {
    pub scope: Scope,
    pub account: AccountId,
    pub frontier: u64,
    pub new_key: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceRevocation {
    pub scope: Scope,
    pub issuer: AccountId,
    pub device: DeviceId,
    pub frontier: u64,
}
