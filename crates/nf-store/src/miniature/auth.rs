use super::{
    error::{MiniatureStoreError, Result},
    model::*,
};
use crate::{StoreError, schema::hash};
use nf_contract::identity::*;
use nf_identity::model::{DeviceProof, MembershipState, ProtectedOperation, Roles, Scope};
use std::{collections::BTreeMap, time::Instant};
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthTicket {
    runtime: [u8; 32],
    sequence: u64,
}
pub struct IssuedChallenge {
    pub ticket: AuthTicket,
    pub template: DeviceProof,
    preimage: [u8; 224],
}
impl IssuedChallenge {
    /// Public transcript data only. The retained private registry remains authoritative.
    pub fn challenge_preimage(&self) -> &[u8; 224] {
        &self.preimage
    }
}
pub struct ProofAttempt {
    pub ticket: AuthTicket,
    pub proof: DeviceProof,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Purpose {
    Lease = 1,
    Claim = 2,
    Prepare = 3,
    Resume = 4,
    Cancel = 5,
    Bootstrap = 6,
    Commit = 7,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Context {
    pub purpose: Purpose,
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub term: AuthorityTerm,
    pub proposed_term: AuthorityTerm,
    pub session: RuntimeSession,
    pub tick: WorldTick,
    pub membership: u64,
    pub actor: AccountId,
    pub device: DeviceId,
    pub binding: [u8; 32],
}
#[derive(Clone)]
pub(crate) struct Evidence {
    pub context: Context,
    pub proof: DeviceProof,
    pub deadline: Instant,
    pub challenge: [u8; 32],
}
pub(crate) struct AuthRuntime {
    runtime: [u8; 32],
    sequence: u64,
    config: AuthConfig,
    issued: BTreeMap<u64, (Context, [u8; 32], Instant)>,
    pub claimed: bool,
    pub leases: Vec<Evidence>,
}
impl AuthRuntime {
    pub fn new(config: AuthConfig) -> Result<Self> {
        Ok(Self {
            runtime: random()?,
            sequence: 0,
            config,
            issued: BTreeMap::new(),
            claimed: false,
            leases: Vec::new(),
        })
    }
    pub fn clear(&mut self) {
        self.issued.clear();
        self.leases.clear();
    }
    pub fn issue(
        &mut self,
        context: Context,
        membership: &MembershipState,
    ) -> Result<IssuedChallenge> {
        self.issued
            .retain(|_, (_, _, deadline)| Instant::now() < *deadline);
        if self.issued.len() >= 64 {
            return Err(StoreError::Backpressure.into());
        }
        let identity = identity(membership, context.actor, context.device)?;
        let sequence = self.sequence.checked_add(1).ok_or(StoreError::Limit)?;
        let nonce = random()?;
        let deadline = Instant::now()
            .checked_add(self.config.lifetime)
            .ok_or(StoreError::Limit)?;
        let preimage = challenge_preimage(context, sequence, nonce);
        let challenge = hash(&preimage);
        self.sequence = sequence;
        self.issued.insert(sequence, (context, challenge, deadline));
        Ok(IssuedChallenge {
            ticket: AuthTicket {
                runtime: self.runtime,
                sequence,
            },
            template: DeviceProof {
                scope: context.scope,
                account: context.actor,
                device: context.device,
                frontier: context.membership,
                peer: identity.peer,
                challenge,
                signature: [0; 64],
            },
            preimage,
        })
    }
    /// Removes the actual issued entry before every semantic or cryptographic check.
    pub fn take(&mut self, attempt: ProofAttempt) -> Result<Evidence> {
        if attempt.ticket.runtime != self.runtime {
            return Err(MiniatureStoreError::Replay);
        }
        let (context, challenge, deadline) = self
            .issued
            .remove(&attempt.ticket.sequence)
            .ok_or(MiniatureStoreError::Replay)?;
        Ok(Evidence {
            context,
            proof: attempt.proof,
            deadline,
            challenge,
        })
    }
    pub fn consume(
        &mut self,
        attempt: ProofAttempt,
        expected: Context,
        membership: &MembershipState,
        authority: bool,
    ) -> Result<Evidence> {
        let evidence = self.take(attempt)?;
        check(&evidence, expected, membership, authority)?;
        Ok(evidence)
    }
}
pub(crate) fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| MiniatureStoreError::Entropy)?;
    Ok(bytes)
}
pub(crate) fn identity(
    state: &MembershipState,
    account: AccountId,
    device: DeviceId,
) -> Result<nf_identity::model::PublicIdentity> {
    let a = state
        .accounts
        .get(&account)
        .ok_or(MiniatureStoreError::Unauthorized)?;
    let d = state
        .devices
        .get(&device)
        .ok_or(MiniatureStoreError::Unauthorized)?;
    if d.account != account || d.revoked {
        return Err(MiniatureStoreError::Unauthorized);
    }
    Ok(nf_identity::model::PublicIdentity {
        account,
        account_key: a.key,
        device,
        device_key: d.key,
        peer: d.peer.clone(),
    })
}
pub(crate) fn verify(
    evidence: &Evidence,
    membership: &MembershipState,
    challenge: &[u8; 32],
    authority: bool,
) -> Result<()> {
    if Instant::now() >= evidence.deadline {
        return Err(MiniatureStoreError::Expired);
    }
    let c = evidence.context;
    if evidence.proof.account != c.actor
        || evidence.proof.device != c.device
        || membership.revision != c.membership
    {
        return Err(MiniatureStoreError::Fenced);
    }
    let public = identity(membership, c.actor, c.device)?;
    membership.authorize(
        &evidence.proof,
        &public.peer,
        challenge,
        c.membership,
        ProtectedOperation::Economic,
    )?;
    if authority
        && !membership.accounts[&c.actor]
            .roles
            .contains(Roles::AUTHORITY_CANDIDATE)
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    Ok(())
}
pub(crate) fn check(
    evidence: &Evidence,
    expected: Context,
    membership: &MembershipState,
    authority: bool,
) -> Result<()> {
    if evidence.context.purpose != expected.purpose {
        return Err(MiniatureStoreError::WrongPurpose);
    }
    if evidence.context != expected {
        return Err(MiniatureStoreError::Fenced);
    }
    verify(evidence, membership, &evidence.challenge, authority)
}
fn challenge_preimage(c: Context, sequence: u64, nonce: [u8; 32]) -> [u8; 224] {
    let mut bytes = Vec::with_capacity(224);
    bytes.extend_from_slice(b"NF-MINI-AUTH-1\0");
    bytes.push(c.purpose as u8);
    bytes.extend_from_slice(c.scope.universe.as_bytes());
    bytes.extend_from_slice(c.scope.history.as_bytes());
    bytes.extend_from_slice(&c.ruleset);
    for value in [
        c.term.0,
        c.proposed_term.0,
        c.session.0,
        c.tick.0,
        c.membership,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(c.actor.as_bytes());
    bytes.extend_from_slice(c.device.as_bytes());
    bytes.extend_from_slice(&c.binding);
    bytes.extend_from_slice(&sequence.to_le_bytes());
    bytes.extend_from_slice(&nonce);
    let mut fixed = [0; 224];
    fixed.copy_from_slice(&bytes);
    fixed
}
