use super::error::Result;
use crate::KnownFrontiers;
use nf_contract::identity::*;
use nf_identity::model::{DeviceProof, PublicIdentity, Scope};
use std::time::Duration;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthConfig {
    pub(crate) lifetime: Duration,
}
impl AuthConfig {
    pub fn new(lifetime: Duration) -> Result<Self> {
        if lifetime < Duration::from_secs(1) || lifetime > Duration::from_secs(30) {
            return Err(crate::StoreError::Limit.into());
        }
        Ok(Self { lifetime })
    }
}
impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            lifetime: Duration::from_secs(30),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureGenesisSpec {
    pub genesis: nf_world::Genesis,
    pub aggregate: AggregateId,
    pub provider: ProviderId,
    pub provider_aggregate: AggregateId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapPolicy {
    pub scope: Scope,
    pub membership_digest: [u8; 32],
    pub owner: PublicIdentity,
    pub controllers: [AccountId; 3],
    pub auth: AuthConfig,
}
/// Signs only the supplied public template. Store verifies its maintained signature.
pub trait BootstrapSigner {
    fn sign(&mut self, template: &DeviceProof) -> Result<DeviceProof>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureKnownFrontiers {
    pub storage: KnownFrontiers,
    pub minimum_authority_term: AuthorityTerm,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureAuthority {
    pub term: AuthorityTerm,
    pub session: RuntimeSession,
    pub account: AccountId,
    pub device: DeviceId,
    pub membership_revision: u64,
}
impl MiniatureAuthority {
    pub fn context(self) -> nf_kernel::AuthorityContext {
        nf_kernel::AuthorityContext {
            term: self.term,
            session: self.session,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MiniatureCancelCause {
    AdmissionChanged,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MiniatureRequestStatus {
    Pending {
        operation: OperationId,
    },
    Committed {
        operation: OperationId,
        sequence: EventSeq,
        rejection: Option<nf_kernel::miniature::MiniatureRejection>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureBoundRequest {
    pub intent: nf_kernel::miniature::MiniatureIntent,
    pub status: MiniatureRequestStatus,
    pub binding_digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureOutbox {
    pub operation: OperationId,
    pub sequence: EventSeq,
    pub batch_digest: [u8; 32],
    pub rejection: Option<nf_kernel::miniature::MiniatureRejection>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureSnapshot {
    pub world: nf_kernel::miniature::MiniatureWorld,
    pub authority: Option<MiniatureAuthority>,
    pub pending: Option<nf_kernel::miniature::MiniatureFrontier>,
    pub known: MiniatureKnownFrontiers,
}

#[derive(Clone, Copy, Debug)]
pub enum ChallengeRequest<'a> {
    Claim { actor: AccountId, device: DeviceId },
    Activity { actor: AccountId, device: DeviceId },
    Prepare(&'a nf_kernel::miniature::MiniatureIntent),
    Resume(RequestId),
    Commit,
    Cancel,
}
