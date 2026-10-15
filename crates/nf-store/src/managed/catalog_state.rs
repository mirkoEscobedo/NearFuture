use super::{bytes::{Encoder, Reader, hash, same}, error::{ManagedError, Result},
    foot::StorageFoot, state::PendingCut};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnerIdentity {
    pub account: AccountId,
    pub device: DeviceId,
    pub incarnation: [u8; 16],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CatalogState {
    pub installation: [u8; 16],
    pub scope: Scope,
    pub phase: u32,
    pub generation: u64,
    pub term: u64,
    pub session: u64,
    pub owner: Option<OwnerIdentity>,
    pub consumed: u64,
    pub selected: Option<[u8; 16]>,
    pub source: [u8; 16],
    pub current: Option<StorageFoot>,
    pub original: StorageFoot,
    pub target: Option<StorageFoot>,
    pub retirement: Option<[u8; 32]>,
    pub pending: Option<[u8; 32]>,
    pub source_policy: Option<PendingCut>,
    pub utc: u64,
    pub registry: [u8; 32],
    pub receipts: [u8; 32],
}
impl CatalogState {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if !(1..=8).contains(&self.phase) || self.generation == 0 || self.term == 0 || self.session == 0
            || self.original.known.scope != self.scope
            || self.current.as_ref().is_some_and(|f| f.known.scope != self.scope)
            || self.target.as_ref().is_some_and(|f| f.known.scope != self.scope)
        { return Err(ManagedError::Corrupt); }
        let mut w = Encoder::new(b"NF-OWNER-CATALOG-STATE-1\0", 16_384)?;
        w.raw(&self.installation)?; w.raw(self.scope.universe.as_bytes())?;
        w.raw(self.scope.history.as_bytes())?; w.u32(self.phase)?;
        w.u64(self.generation)?; w.u64(self.term)?; w.u64(self.session)?;
        w.option(self.owner.as_ref(), |w, owner| {
            w.raw(owner.account.as_bytes())?; w.raw(owner.device.as_bytes())?; w.raw(&owner.incarnation)
        })?;
        w.u64(self.consumed)?; w.option(self.selected, |w, id| w.raw(&id))?;
        w.raw(&self.source)?;
        w.option(self.current.as_ref(), |w, f| w.blob(f.canonical_bytes(), 2048))?;
        w.blob(self.original.canonical_bytes(), 2048)?;
        w.option(self.target.as_ref(), |w, f| w.blob(f.canonical_bytes(), 2048))?;
        w.option(self.retirement, |w, sha| w.raw(&sha))?;
        w.option(self.pending, |w, sha| w.raw(&sha))?;
        w.option(self.source_policy, |w, cut| cut.write(w))?;
        w.u64(self.utc)?; w.raw(&self.registry)?; w.raw(&self.receipts)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-CATALOG-STATE-1\0", 16_384)?;
        let installation = r.fixed()?;
        let scope = Scope { universe: UniverseId::from_bytes(r.fixed()?), history: HistoryId::from_bytes(r.fixed()?) };
        let value = Self {
            installation, scope, phase: r.u32()?, generation: r.u64()?, term: r.u64()?, session: r.u64()?,
            owner: r.option(|r| Ok(OwnerIdentity { account: AccountId::from_bytes(r.fixed()?),
                device: DeviceId::from_bytes(r.fixed()?), incarnation: r.fixed()? }))?,
            consumed: r.u64()?, selected: r.option(Reader::fixed)?, source: r.fixed()?,
            current: r.option(|r| StorageFoot::decode(r.blob(2048)?))?,
            original: StorageFoot::decode(r.blob(2048)?)?,
            target: r.option(|r| StorageFoot::decode(r.blob(2048)?))?,
            retirement: r.option(Reader::fixed)?, pending: r.option(Reader::fixed)?,
            source_policy: r.option(PendingCut::read)?, utc: r.u64()?, registry: r.fixed()?, receipts: r.fixed()?,
        };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
    pub fn digest(&self) -> Result<[u8; 32]> { Ok(hash(&self.encode()?)) }
}

pub(super) fn journal_head(previous: [u8; 32], revision: u64, kind: u32, action: &[u8],
    before: [u8; 32], after: [u8; 32], registry: [u8; 32]) -> Result<[u8; 32]> {
    if !(1..=8).contains(&kind) { return Err(ManagedError::Corrupt); }
    let mut w = Encoder::new(b"NF-OWNER-CATALOG-JOURNAL-1\0", 256)?;
    w.raw(&previous)?; w.u64(revision)?; w.u32(kind)?;
    w.raw(&hash(action))?; w.raw(&before)?; w.raw(&after)?; w.raw(&registry)?;
    Ok(hash(&w.finish()))
}
