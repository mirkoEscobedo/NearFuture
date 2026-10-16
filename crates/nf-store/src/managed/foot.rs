use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
    state::PendingCut,
};
use crate::KnownFrontiers;
use nf_contract::identity::{EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageFoot {
    pub(super) profile: u32,
    pub(super) schema: [u8; 32],
    pub(super) known: KnownFrontiers,
    pub(super) core: [u8; 32],
    pub(super) member: Option<[u8; 32]>,
    pub(super) state: [u8; 32],
    pub(super) head: [u8; 32],
    pub(super) lineage: Option<[u8; 32]>,
    pub(super) quota: u32,
    pub(super) policy: Option<PendingCut>,
    pub(super) cancellations: [u8; 32],
    pub(super) checkpoint: CheckpointFoot,
    pub(super) rows: [u8; 32],
    bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CheckpointFoot {
    pub revision: u64,
    pub event: u64,
    pub membership: Option<u64>,
    pub state: [u8; 32],
    pub head: [u8; 32],
    pub quota: u32,
}
impl CheckpointFoot {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::plain(93);
        w.u64(self.revision)?;
        w.u64(self.event)?;
        w.option(self.membership, |w, n| w.u64(n))?;
        w.raw(&self.state)?;
        w.raw(&self.head)?;
        w.u32(self.quota)?;
        Ok(w.finish())
    }
    pub fn read(r: &mut Reader<'_>) -> Result<Self> {
        let value = Self {
            revision: r.u64()?,
            event: r.u64()?,
            membership: r.option(Reader::u64)?,
            state: r.fixed()?,
            head: r.fixed()?,
            quota: r.u32()?,
        };
        if !(1..=65_536).contains(&value.quota) {
            return Err(ManagedError::Corrupt);
        }
        Ok(value)
    }
}
impl StorageFoot {
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn known_frontiers(&self) -> KnownFrontiers {
        self.known
    }
    pub(super) fn encode_fields(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::new(b"NF-OWNER-STORAGE-1\0", 2048)?;
        w.u32(self.profile)?;
        w.raw(&self.schema)?;
        w.raw(self.known.scope.universe.as_bytes())?;
        w.raw(self.known.scope.history.as_bytes())?;
        w.u64(self.known.store_revision)?;
        w.u64(self.known.event_sequence.0)?;
        w.option(self.known.membership_revision, |w, n| w.u64(n))?;
        w.raw(&self.core)?;
        w.option(self.member, |w, sha| w.raw(&sha))?;
        w.raw(&self.state)?;
        w.raw(&self.head)?;
        w.option(self.lineage, |w, sha| w.raw(&sha))?;
        w.u32(self.quota)?;
        w.option(self.policy, |w, cut| cut.write(w))?;
        w.raw(&self.cancellations)?;
        w.raw(&self.checkpoint.encode()?)?;
        w.raw(&self.rows)?;
        Ok(w.finish())
    }
    pub(super) fn new(mut value: Self) -> Result<Self> {
        if !matches!(value.profile, 1 | 4 | 5)
            || !(1..=65_536).contains(&value.quota)
            || value.member.is_some() != value.known.membership_revision.is_some()
            || (value.profile == 4) != value.lineage.is_some()
            || value.checkpoint.revision > value.known.store_revision
            || value.checkpoint.event > value.known.event_sequence.0
            || value.checkpoint.membership > value.known.membership_revision
        {
            return Err(ManagedError::Corrupt);
        }
        value.bytes = value.encode_fields()?;
        Ok(value)
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-STORAGE-1\0", 2048)?;
        let profile = r.u32()?;
        let schema = r.fixed()?;
        let scope = Scope {
            universe: UniverseId::from_bytes(r.fixed()?),
            history: HistoryId::from_bytes(r.fixed()?),
        };
        let known = KnownFrontiers {
            scope,
            store_revision: r.u64()?,
            event_sequence: EventSeq(r.u64()?),
            membership_revision: r.option(Reader::u64)?,
        };
        let value = Self {
            profile,
            schema,
            known,
            core: r.fixed()?,
            member: r.option(Reader::fixed)?,
            state: r.fixed()?,
            head: r.fixed()?,
            lineage: r.option(Reader::fixed)?,
            quota: r.u32()?,
            policy: r.option(PendingCut::read)?,
            cancellations: r.fixed()?,
            checkpoint: CheckpointFoot::read(&mut r)?,
            rows: r.fixed()?,
            bytes: Vec::new(),
        };
        r.finish()?;
        let value = Self::new(value)?;
        same(bytes, value.canonical_bytes())?;
        Ok(value)
    }
    pub(super) fn from_parts(
        profile: u32,
        schema: [u8; 32],
        known: KnownFrontiers,
        core: [u8; 32],
        member: Option<[u8; 32]>,
        state: [u8; 32],
        head: [u8; 32],
        lineage: Option<[u8; 32]>,
        quota: u32,
        policy: Option<PendingCut>,
        cancellations: [u8; 32],
        checkpoint: CheckpointFoot,
        rows: [u8; 32],
    ) -> Result<Self> {
        Self::new(Self {
            profile,
            schema,
            known,
            core,
            member,
            state,
            head,
            lineage,
            quota,
            policy,
            cancellations,
            checkpoint,
            rows,
            bytes: Vec::new(),
        })
    }
}
