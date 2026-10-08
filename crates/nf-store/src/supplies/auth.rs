use super::{
    ChallengeRequest,
    error::{Result, SuppliesStoreError},
};
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::model::{DeviceProof, IdentityError, MembershipState, ProtectedOperation};
use nf_kernel::supplies::{
    balance_digest, burn_binding, issuance_binding, reserve_binding, status_digest,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthTicket {
    runtime: [u8; 32],
    sequence: u64,
}
pub struct IssuedChallenge {
    pub ticket: AuthTicket,
    pub template: DeviceProof,
}
pub struct ProofAttempt {
    pub ticket: AuthTicket,
    pub proof: DeviceProof,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Purpose {
    Issue = 1,
    Balance = 2,
    Burn = 3,
    Status = 4,
    Reserve = 5,
}
#[derive(Clone, Copy, Eq, PartialEq)]
struct Context {
    purpose: Purpose,
    actor: AccountId,
    device: DeviceId,
    membership: u64,
    digest: [u8; 32],
}
struct Issued {
    context: Context,
    challenge: [u8; 32],
    deadline: Instant,
}
pub(super) struct Evidence {
    issued: Issued,
    proof: DeviceProof,
}
pub(super) struct AuthRuntime {
    runtime: [u8; 32],
    sequence: u64,
    issued: BTreeMap<u64, Issued>,
}
fn context(request: ChallengeRequest<'_>, membership: u64) -> Context {
    match request {
        ChallengeRequest::Issue(i) => Context {
            purpose: Purpose::Issue,
            actor: i.actor,
            device: i.device,
            membership,
            digest: issuance_binding(i).digest(),
        },
        ChallengeRequest::Burn(b) => Context {
            purpose: Purpose::Burn,
            actor: b.actor,
            device: b.device,
            membership,
            digest: burn_binding(b).digest(),
        },
        ChallengeRequest::Reserve(r) => Context {
            purpose: Purpose::Reserve,
            actor: r.actor,
            device: r.device,
            membership,
            digest: reserve_binding(r).digest(),
        },
        ChallengeRequest::Status(q) => Context {
            purpose: Purpose::Status,
            actor: q.actor,
            device: q.device,
            membership,
            digest: status_digest(q),
        },
        ChallengeRequest::Balance(q) => Context {
            purpose: Purpose::Balance,
            actor: q.actor,
            device: q.device,
            membership,
            digest: balance_digest(q),
        },
    }
}
fn random() -> Result<[u8; 32]> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| SuppliesStoreError::Entropy)?;
    Ok(bytes)
}
impl AuthRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runtime: random()?,
            sequence: 0,
            issued: BTreeMap::new(),
        })
    }
    pub fn issue(
        &mut self,
        request: ChallengeRequest<'_>,
        state: &MembershipState,
        policy: [u8; 32],
    ) -> Result<IssuedChallenge> {
        self.issued.retain(|_, v| Instant::now() < v.deadline);
        if self.issued.len() >= 64 {
            return Err(crate::StoreError::Backpressure.into());
        }
        let context = context(request, state.revision);
        let device = state
            .devices
            .get(&context.device)
            .ok_or(IdentityError::UnknownDevice)?;
        if device.account != context.actor || device.revoked {
            return Err(IdentityError::Revoked.into());
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(crate::StoreError::Limit)?;
        let mut hash = Sha256::new();
        hash.update(b"NF-SUPPLIES-AUTH-1\0");
        hash.update(self.runtime);
        hash.update(sequence.to_le_bytes());
        hash.update(random()?);
        hash.update(policy);
        hash.update([context.purpose as u8]);
        hash.update(context.digest);
        hash.update(context.membership.to_le_bytes());
        let challenge: [u8; 32] = hash.finalize().into();
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(5))
            .ok_or(crate::StoreError::Limit)?;
        self.sequence = sequence;
        self.issued.insert(
            sequence,
            Issued {
                context,
                challenge,
                deadline,
            },
        );
        Ok(IssuedChallenge {
            ticket: AuthTicket {
                runtime: self.runtime,
                sequence,
            },
            template: DeviceProof {
                scope: state.scope,
                account: context.actor,
                device: context.device,
                frontier: state.revision,
                peer: device.peer.clone(),
                challenge,
                signature: [0; 64],
            },
        })
    }
    /// Consumes the actual registry entry before fresh storage or semantic checks.
    pub fn take(&mut self, attempt: ProofAttempt) -> Result<Evidence> {
        if attempt.ticket.runtime != self.runtime {
            return Err(SuppliesStoreError::Replay);
        }
        let issued = self
            .issued
            .remove(&attempt.ticket.sequence)
            .ok_or(SuppliesStoreError::Replay)?;
        Ok(Evidence {
            issued,
            proof: attempt.proof,
        })
    }
}
pub(super) fn live(evidence: &Evidence) -> Result<()> {
    if Instant::now() >= evidence.issued.deadline {
        return Err(SuppliesStoreError::Expired);
    }
    Ok(())
}
pub(super) fn verify(
    evidence: &Evidence,
    request: ChallengeRequest<'_>,
    state: &MembershipState,
) -> Result<()> {
    if Instant::now() >= evidence.issued.deadline {
        return Err(SuppliesStoreError::Expired);
    }
    let expected = context(request, state.revision);
    if evidence.issued.context != expected
        || evidence.proof.account != expected.actor
        || evidence.proof.device != expected.device
    {
        return Err(IdentityError::Frontier.into());
    }
    let device = state
        .devices
        .get(&expected.device)
        .ok_or(IdentityError::UnknownDevice)?;
    let operation = match expected.purpose {
        Purpose::Issue | Purpose::Burn => ProtectedOperation::Administration,
        Purpose::Balance | Purpose::Status | Purpose::Reserve => ProtectedOperation::Economic,
    };
    state.authorize(
        &evidence.proof,
        &device.peer,
        &evidence.issued.challenge,
        state.revision,
        operation,
    )?;
    if Instant::now() >= evidence.issued.deadline {
        return Err(SuppliesStoreError::Expired);
    }
    Ok(())
}
