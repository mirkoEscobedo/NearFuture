use super::{bytes::{Encoder, Reader, same}, error::{ManagedError, Result}, foot::StorageFoot,
    state::QuiescePolicy};
use crate::KnownFrontiers;
use nf_contract::identity::{EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;
use std::path::PathBuf;

#[derive(Clone)]
pub(super) enum Command {
    Quiesce { foot: StorageFoot, policy: QuiescePolicy, cutoff: u64, delta: [u8; 32] },
    Register { registry: [u8; 32], delta: [u8; 32] },
    Backup { path: PathBuf, minimum: KnownFrontiers, foot: StorageFoot, max_bytes: u64,
        pages: u32, steps: u32 },
}
impl Command {
    pub fn purpose(&self) -> u32 {
        match self { Self::Quiesce { .. } => 11, Self::Register { .. } => 12, Self::Backup { .. } => 15 }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::new(b"NF-OWNER-CATALOG-COMMAND-1\0", 32_768)?;
        w.u32(self.purpose())?;
        match self {
            Self::Quiesce { foot, policy, cutoff, delta } => {
                if foot.profile != 1 || foot.policy.is_some() { return Err(ManagedError::NotDrained); }
                w.byte(1)?; w.blob(foot.canonical_bytes(), 2048)?; w.byte(policy.tag())?;
                w.u64(*cutoff)?; w.raw(delta)?;
            },
            Self::Register { registry, delta } => { w.byte(2)?; w.raw(registry)?; w.raw(delta)?; },
            Self::Backup { path, minimum, foot, max_bytes, pages, steps } => {
                let text = path.to_str().ok_or(ManagedError::Scope)?;
                if !path.is_absolute() || text.as_bytes().contains(&0) || minimum.scope != foot.known.scope
                    || minimum.store_revision > foot.known.store_revision || minimum.event_sequence > foot.known.event_sequence
                    || minimum.membership_revision > foot.known.membership_revision || *pages == 0 || *pages > foot.quota
                    || *max_bytes != u64::from(foot.quota) * 4096 || *steps != pages.div_ceil(100) || *steps > 656
                { return Err(ManagedError::Limit); }
                w.blob(text.as_bytes(), 4096)?; w.byte(1)?;
                w.raw(minimum.scope.universe.as_bytes())?; w.raw(minimum.scope.history.as_bytes())?;
                w.u64(minimum.store_revision)?; w.u64(minimum.event_sequence.0)?;
                w.option(minimum.membership_revision, |w, n| w.u64(n))?;
                w.blob(foot.canonical_bytes(), 2048)?; w.u64(*max_bytes)?;
                w.u32(*pages)?; w.u32(100)?; w.u32(*steps)?;
            },
        }
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-CATALOG-COMMAND-1\0", 32_768)?;
        let value = match r.u32()? {
            11 => {
                if r.byte()? != 1 { return Err(ManagedError::Corrupt); }
                Self::Quiesce { foot: StorageFoot::decode(r.blob(2048)?)?, policy: match r.byte()? {
                    1 => QuiescePolicy::Drain, 2 => QuiescePolicy::RequireAlreadyDrained, _ => return Err(ManagedError::Corrupt),
                }, cutoff: r.u64()?, delta: r.fixed()? }
            },
            12 => {
                if r.byte()? != 2 { return Err(ManagedError::Corrupt); }
                Self::Register { registry: r.fixed()?, delta: r.fixed()? }
            },
            15 => {
                let path = PathBuf::from(std::str::from_utf8(r.blob(4096)?).map_err(|_| ManagedError::Corrupt)?);
                if r.byte()? != 1 { return Err(ManagedError::Corrupt); }
                let scope = Scope { universe: UniverseId::from_bytes(r.fixed()?), history: HistoryId::from_bytes(r.fixed()?) };
                let minimum = KnownFrontiers { scope, store_revision: r.u64()?, event_sequence: EventSeq(r.u64()?),
                    membership_revision: r.option(Reader::u64)? };
                let foot = StorageFoot::decode(r.blob(2048)?)?; let max_bytes = r.u64()?;
                let pages = r.u32()?; if r.u32()? != 100 { return Err(ManagedError::Corrupt); }
                Self::Backup { path, minimum, foot, max_bytes, pages, steps: r.u32()? }
            },
            _ => return Err(ManagedError::OperationConflict),
        };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
}
