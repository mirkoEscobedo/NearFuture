use super::{
    bytes::{hash, next},
    challenge::{Binding, Challenge, Times},
    error::{ManagedError, Result},
};
use nf_identity::model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity, Roles};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Entry {
    binding: Binding,
    transcript: Vec<u8>,
    deadline: Instant,
    issued: u64,
    expires: u64,
    unsigned: Option<Vec<u8>>,
}
pub(super) struct Book {
    id: [u8; 32],
    sequence: u64,
    entries: BTreeMap<[u8; 32], Entry>,
}
pub(super) fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0_u8; N];
    getrandom::fill(&mut bytes).map_err(|_| ManagedError::Storage(crate::StoreError::Io))?;
    if bytes == [0; N] {
        return Err(ManagedError::Storage(crate::StoreError::Io));
    }
    Ok(bytes)
}
pub(super) fn now_ms() -> Result<u64> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ManagedError::Expired)?;
    time.as_millis().try_into().map_err(|_| ManagedError::Limit)
}
pub(super) fn eligible(
    member: &MembershipState,
    public: &PublicIdentity,
    admin: bool,
) -> Result<()> {
    nf_identity::codec::encode_state(member)?;
    let account = member
        .accounts
        .get(&public.account)
        .ok_or(ManagedError::Unauthorized)?;
    let device = member
        .devices
        .get(&public.device)
        .ok_or(ManagedError::Unauthorized)?;
    if public.peer.len() > 128
        || member.owner != public.account
        || account.key != public.account_key
        || device.key != public.device_key
        || device.account != public.account
        || device.revoked
        || device.peer != public.peer
        || !account.roles.contains(Roles::AUTHORITY_CANDIDATE)
        || (admin && !account.roles.contains(Roles::MODERATOR))
    {
        return Err(ManagedError::Unauthorized);
    }
    Ok(())
}
impl Book {
    pub fn new() -> Result<Self> {
        Ok(Self {
            id: random()?,
            sequence: 0,
            entries: BTreeMap::new(),
        })
    }
    pub fn times(&mut self, cutoff: u64, floor: u64) -> Result<Times> {
        self.entries
            .retain(|_, entry| Instant::now() < entry.deadline);
        if self.entries.len() >= 128 {
            return Err(ManagedError::Limit);
        }
        let issued = now_ms()?;
        if issued < floor || cutoff <= issued {
            return Err(ManagedError::Expired);
        }
        let expires = issued
            .checked_add(20_000)
            .ok_or(ManagedError::Limit)?
            .min(cutoff);
        self.sequence = next(self.sequence)?;
        Ok(Times {
            auth_sequence: self.sequence,
            nonce: random()?,
            issued,
            expires,
        })
    }
    pub fn issue(
        &mut self,
        binding: Binding,
        times: &Times,
        transcript: Vec<u8>,
        admission: Vec<u8>,
        unsigned: Vec<u8>,
        delta: Vec<u8>,
    ) -> Result<Challenge> {
        if transcript.len() > 4096
            || admission.len() > 172_002
            || unsigned.len() > 1_114_112
            || delta.len() > 131_072
            || times.auth_sequence != self.sequence
            || times.expires <= times.issued
            || times.expires - times.issued > 20_000
        {
            return Err(ManagedError::Limit);
        }
        // Validate the actual bytes retained/returned at this private signing boundary.
        match &binding {
            Binding::Prospective {
                purpose,
                candidate,
                input,
            } => {
                let admitted = super::admission::Input::decode(&admission)?;
                let prospective = super::prospective::Prospective::decode(&transcript)?;
                if admitted.operation.purpose() != *purpose
                    || admitted.installation != candidate.installation
                    || admitted.scope != prospective.scope
                    || admitted.foot != prospective.foot
                    || prospective.purpose != *purpose
                    || prospective.candidate != *candidate
                    || prospective.installation != admitted.installation
                    || prospective.input != *input
                    || hash(&admission) != *input
                    || prospective.times.auth_sequence != times.auth_sequence
                    || prospective.times.nonce != times.nonce
                    || prospective.times.issued != times.issued
                    || prospective.times.expires != times.expires
                {
                    return Err(ManagedError::Authentication);
                }
            }
            _ if !admission.is_empty() => return Err(ManagedError::OperationConflict),
            _ => {}
        }
        let current = now_ms()?;
        if current < times.issued || current >= times.expires {
            return Err(ManagedError::Expired);
        }
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(times.expires - current))
            .ok_or(ManagedError::Limit)?;
        let digest = hash(&transcript);
        if self.entries.contains_key(&digest) || self.entries.len() >= 128 {
            return Err(ManagedError::Replay);
        }
        self.entries.insert(
            digest,
            Entry {
                binding,
                transcript: transcript.clone(),
                deadline,
                issued: times.issued,
                expires: times.expires,
                unsigned: if unsigned.len() <= 32_768 {
                    Some(unsigned.clone())
                } else {
                    None
                },
            },
        );
        Ok(Challenge {
            transcript,
            admission,
            unsigned,
            delta,
            book: self.id,
        })
    }
    pub fn consume(
        &mut self,
        proof: &DeviceProof,
        member: &MembershipState,
        operation: ProtectedOperation,
        require_owner: bool,
    ) -> Result<(Binding, Vec<u8>, Option<Vec<u8>>)> {
        let entry = self
            .entries
            .remove(&proof.challenge)
            .ok_or(ManagedError::Replay)?;
        let utc = now_ms()?;
        if Instant::now() >= entry.deadline || utc < entry.issued || utc >= entry.expires {
            return Err(ManagedError::Expired);
        }
        let expected_person = match &entry.binding {
            Binding::Owned(value) => (value.stamp.account, value.stamp.device),
            Binding::Prospective { candidate, .. } => (candidate.account, candidate.device),
            Binding::Principal(value) => (value.actor, value.device),
        };
        if (proof.account, proof.device) != expected_person {
            return Err(ManagedError::Authentication);
        }
        if require_owner {
            let account = member
                .accounts
                .get(&proof.account)
                .ok_or(ManagedError::Unauthorized)?;
            if member.owner != proof.account || !account.roles.contains(Roles::AUTHORITY_CANDIDATE)
            {
                return Err(ManagedError::Unauthorized);
            }
        }
        let device = member
            .devices
            .get(&proof.device)
            .ok_or(ManagedError::Unauthorized)?;
        member.authorize(
            proof,
            &device.peer,
            &hash(&entry.transcript),
            member.revision,
            operation,
        )?;
        Ok((entry.binding, entry.transcript, entry.unsigned))
    }
    pub fn consume_principal(
        &mut self,
        proof: &super::challenge::PrincipalProof,
        member: &MembershipState,
    ) -> Result<super::challenge::Principal> {
        if proof.book != self.id {
            return Err(ManagedError::Replay);
        }
        let (binding, transcript, _) =
            self.consume(&proof.proof, member, ProtectedOperation::Economic, false)?;
        if transcript != proof.transcript {
            return Err(ManagedError::Authentication);
        }
        match binding {
            Binding::Principal(value) => Ok(value),
            _ => Err(ManagedError::OperationConflict),
        }
    }
    pub fn inspect_principal(
        &self,
        proof: &super::challenge::PrincipalProof,
    ) -> Result<super::challenge::Principal> {
        if proof.book != self.id {
            return Err(ManagedError::Replay);
        }
        let entry = self
            .entries
            .get(&proof.proof.challenge)
            .ok_or(ManagedError::Replay)?;
        let utc = now_ms()?;
        if entry.transcript != proof.transcript
            || hash(&entry.transcript) != proof.proof.challenge
            || Instant::now() >= entry.deadline
            || utc < entry.issued
            || utc >= entry.expires
        {
            return Err(ManagedError::Expired);
        }
        match &entry.binding {
            Binding::Principal(principal) => Ok(principal.clone()),
            _ => Err(ManagedError::OperationConflict),
        }
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
