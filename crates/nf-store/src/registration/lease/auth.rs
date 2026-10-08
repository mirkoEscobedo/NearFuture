use super::{
    codec::request_digest,
    model::{AdmitLease, LeaseError, Result},
};
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::model::{DeviceProof, IdentityError, MembershipState, ProtectedOperation};
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
struct Context {
    account: AccountId,
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

fn context(request: &AdmitLease, membership: u64) -> Context {
    Context {
        account: request.binding.account,
        device: request.device,
        membership,
        digest: request_digest(request),
    }
}
fn random() -> Result<[u8; 32]> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| LeaseError::Entropy)?;
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
        request: &AdmitLease,
        membership: &MembershipState,
        policy_digest: [u8; 32],
    ) -> Result<IssuedChallenge> {
        self.issued
            .retain(|_, issued| Instant::now() < issued.deadline);
        if self.issued.len() >= 64 {
            return Err(crate::StoreError::Backpressure.into());
        }
        let context = context(request, membership.revision);
        let device = membership
            .devices
            .get(&context.device)
            .ok_or(IdentityError::UnknownDevice)?;
        if device.account != context.account || device.revoked {
            return Err(IdentityError::Revoked.into());
        }
        let sequence = self.sequence.checked_add(1).ok_or(LeaseError::Limit)?;
        let mut hash = Sha256::new();
        hash.update(b"NF-HEADLESS-LEASE-AUTH-1\0");
        hash.update(self.runtime);
        hash.update(sequence.to_be_bytes());
        hash.update(random()?);
        hash.update(policy_digest);
        hash.update([1]); // Closed admission purpose, distinct from registration.
        hash.update(context.digest);
        hash.update(context.membership.to_be_bytes());
        let challenge = hash.finalize().into();
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(5))
            .ok_or(LeaseError::Limit)?;
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
                scope: membership.scope,
                account: context.account,
                device: context.device,
                frontier: membership.revision,
                peer: device.peer.clone(),
                challenge,
                signature: [0; 64],
            },
        })
    }
    pub fn take(&mut self, attempt: ProofAttempt) -> Result<Evidence> {
        if attempt.ticket.runtime != self.runtime {
            return Err(LeaseError::Replay);
        }
        let issued = self
            .issued
            .remove(&attempt.ticket.sequence)
            .ok_or(LeaseError::Replay)?;
        Ok(Evidence {
            issued,
            proof: attempt.proof,
        })
    }
}
pub(super) fn live(evidence: &Evidence) -> Result<()> {
    if Instant::now() >= evidence.issued.deadline {
        return Err(LeaseError::Expired);
    }
    Ok(())
}
pub(super) fn verify(
    evidence: &Evidence,
    request: &AdmitLease,
    membership: &MembershipState,
) -> Result<()> {
    live(evidence)?;
    let expected = context(request, membership.revision);
    if evidence.issued.context != expected
        || evidence.proof.account != expected.account
        || evidence.proof.device != expected.device
    {
        return Err(IdentityError::Frontier.into());
    }
    let device = membership
        .devices
        .get(&expected.device)
        .ok_or(IdentityError::UnknownDevice)?;
    membership.authorize(
        &evidence.proof,
        &device.peer,
        &evidence.issued.challenge,
        membership.revision,
        ProtectedOperation::Economic,
    )?;
    live(evidence)
}
