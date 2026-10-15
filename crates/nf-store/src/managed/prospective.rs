use super::{bytes::{Encoder, Reader, same}, challenge::Times, error::{ManagedError, Result},
    foot::StorageFoot, stamp::{CatalogFrontier, OwnerStamp}};
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::model::Scope;

#[derive(Clone)]
pub(super) struct Prospective {
    pub purpose: u32,
    pub installation: [u8; 16],
    pub scope: Scope,
    pub catalog: Option<CatalogFrontier>,
    pub candidate: OwnerStamp,
    pub foot: StorageFoot,
    pub peer: Vec<u8>,
    pub input: [u8; 32],
    pub times: Times,
}
impl Prospective {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if !matches!(self.purpose, 9 | 10 | 13 | 14) || (self.purpose == 9) != self.catalog.is_none()
            || self.installation != self.candidate.installation || self.scope != self.foot.known.scope
            || self.times.auth_sequence == 0 || self.times.expires <= self.times.issued
            || self.times.expires - self.times.issued > 20_000
        { return Err(ManagedError::Corrupt); }
        let mut w = Encoder::new(b"NF-OWNER-PROSPECTIVE-1\0", 4096)?;
        w.u32(self.purpose)?; w.raw(&self.installation)?;
        w.raw(self.scope.universe.as_bytes())?; w.raw(self.scope.history.as_bytes())?;
        w.option(self.catalog, |w, c| { w.u64(c.revision)?; w.raw(&c.state_digest)?; w.raw(&c.head) })?;
        w.raw(self.candidate.canonical_bytes())?; w.blob(self.foot.canonical_bytes(), 2048)?;
        w.blob(&self.peer, 128)?; w.raw(&self.input)?; w.u64(self.times.auth_sequence)?;
        w.raw(&self.times.nonce)?; w.u64(self.times.issued)?; w.u64(self.times.expires)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-PROSPECTIVE-1\0", 4096)?;
        let purpose = r.u32()?; let installation = r.fixed()?;
        let scope = Scope { universe: UniverseId::from_bytes(r.fixed()?), history: HistoryId::from_bytes(r.fixed()?) };
        let value = Self { purpose, installation, scope,
            catalog: r.option(|r| Ok(CatalogFrontier { revision: r.u64()?, state_digest: r.fixed()?, head: r.fixed()? }))?,
            candidate: OwnerStamp::read(&mut r)?, foot: StorageFoot::decode(r.blob(2048)?)?,
            peer: r.blob(128)?.to_vec(), input: r.fixed()?, times: Times { auth_sequence: r.u64()?,
                nonce: r.fixed()?, issued: r.u64()?, expires: r.u64()? } };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
}
