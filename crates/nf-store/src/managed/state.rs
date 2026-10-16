use super::{
    bytes::{Encoder, Reader, hash, same},
    cancellation::{AdministrativeCancellation, encode_map},
    error::{ManagedError, Result},
};
use crate::model::{RequestStatus, State};
use nf_contract::identity::RequestId;
use nf_identity::model::MembershipState;
use std::collections::BTreeMap;

pub(super) const FULL_STATE_CAP: usize = 1_884_160;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingPolicy {
    Drain,
    Cancel,
}
impl PendingPolicy {
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Drain => 1,
            Self::Cancel => 2,
        }
    }
    pub(super) fn decode(tag: u8) -> Result<Self> {
        match tag {
            1 => Ok(Self::Drain),
            2 => Ok(Self::Cancel),
            _ => Err(ManagedError::Corrupt),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuiescePolicy {
    Drain,
    RequireAlreadyDrained,
}
impl QuiescePolicy {
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Drain => 1,
            Self::RequireAlreadyDrained => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingCut {
    pub policy: PendingPolicy,
    pub admitted: u64,
    pub cutoff: u64,
}
impl PendingCut {
    pub fn write(self, w: &mut Encoder) -> Result<()> {
        w.byte(self.policy.tag())?;
        w.u64(self.admitted)?;
        w.u64(self.cutoff)
    }
    pub fn read(r: &mut Reader<'_>) -> Result<Self> {
        let cut = Self {
            policy: PendingPolicy::decode(r.byte()?)?,
            admitted: r.u64()?,
            cutoff: r.u64()?,
        };
        if cut.cutoff <= cut.admitted {
            return Err(ManagedError::Corrupt);
        }
        Ok(cut)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FullState {
    pub core: State,
    pub member: Option<MembershipState>,
    pub quota: u32,
    pub policy: Option<PendingCut>,
    pub cancellations: BTreeMap<RequestId, AdministrativeCancellation>,
}
impl FullState {
    pub fn validate(&self, revision: u64) -> Result<()> {
        self.core.validate()?;
        if !(1..=65_536).contains(&self.quota)
            || self.core.pending.is_some() != self.policy.is_some()
            || self.cancellations.len() > 4096
        {
            return Err(ManagedError::Corrupt);
        }
        if let Some(cut) = self.policy {
            if cut.cutoff <= cut.admitted {
                return Err(ManagedError::Corrupt);
            }
        }
        if let Some(member) = &self.member {
            if member.scope != self.core.scope() {
                return Err(ManagedError::Scope);
            }
            nf_identity::codec::encode_state(member)?;
        }
        for (request, cancellation) in &self.cancellations {
            let record = self
                .core
                .requests
                .get(request)
                .ok_or(ManagedError::Corrupt)?;
            if *request != cancellation.request
                || record.intent.operation != cancellation.operation
                || record.intent.job != cancellation.job
                || cancellation.revision > revision
                || !matches!(record.status, RequestStatus::Committed { .. })
                || !self
                    .core
                    .world
                    .outcomes()
                    .iter()
                    .any(|o| o.operation == cancellation.operation && o.job == cancellation.job)
            {
                return Err(ManagedError::Corrupt);
            }
            AdministrativeCancellation::decode(&cancellation.canonical_bytes())?;
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate(u64::MAX)?;
        let core = crate::codec::encode(&self.core)?;
        let member = self
            .member
            .as_ref()
            .map(nf_identity::codec::encode_state)
            .transpose()?;
        let mut w = Encoder::new(b"NF-ACTIVE-STATE-1\0", FULL_STATE_CAP)?;
        w.blob(&core, 1_048_576)?;
        w.option(member.as_deref(), |w, bytes| w.blob(bytes, 262_144))?;
        w.u32(self.quota)?;
        w.option(self.policy, |w, cut| cut.write(w))?;
        w.raw(&encode_map(self.cancellations.values())?)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8], revision: u64) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-ACTIVE-STATE-1\0", FULL_STATE_CAP)?;
        let core = crate::codec::decode(r.blob(1_048_576)?)?;
        let member = r.option(|r| Ok(nf_identity::codec::decode_state(r.blob(262_144)?)?))?;
        let quota = r.u32()?;
        let policy = r.option(PendingCut::read)?;
        let count = r.u32()?;
        if count > 4096 {
            return Err(ManagedError::Limit);
        }
        let mut cancellations = BTreeMap::new();
        let mut previous = None;
        for _ in 0..count {
            let value = AdministrativeCancellation::decode(r.take(138)?)?;
            if previous.is_some_and(|old| old >= value.request) {
                return Err(ManagedError::Corrupt);
            }
            previous = Some(value.request);
            cancellations.insert(value.request, value);
        }
        r.finish()?;
        let value = Self {
            core,
            member,
            quota,
            policy,
            cancellations,
        };
        value.validate(revision)?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
    pub fn cancellation_digest(&self) -> Result<[u8; 32]> {
        Ok(hash(&encode_map(self.cancellations.values())?))
    }
}
