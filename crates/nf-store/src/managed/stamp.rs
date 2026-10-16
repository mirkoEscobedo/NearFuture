use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
};
use nf_contract::identity::{AccountId, DeviceId};

/// Observed data; possession does not grant a current owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerStamp {
    pub(super) installation: [u8; 16],
    pub(super) generation: u64,
    pub(super) account: AccountId,
    pub(super) device: DeviceId,
    pub(super) incarnation: [u8; 16],
    pub(super) term: u64,
    pub(super) session: u64,
    pub(super) profile: u32,
    pub(super) schema: [u8; 32],
    pub(super) ruleset: [u8; 32],
    pub(super) membership_revision: Option<u64>,
    pub(super) membership_digest: [u8; 32],
    pub(super) bytes: Vec<u8>,
}
impl OwnerStamp {
    /// Decode immutable worker-envelope data only; only the private live gate can admit it.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self> {
        Self::decode(bytes)
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(super) fn encode_fields(&self) -> Result<Vec<u8>> {
        let mut w = Encoder::plain(197);
        w.raw(&self.installation)?;
        w.u64(self.generation)?;
        w.raw(self.account.as_bytes())?;
        w.raw(self.device.as_bytes())?;
        w.raw(&self.incarnation)?;
        w.u64(self.term)?;
        w.u64(self.session)?;
        w.u32(self.profile)?;
        w.raw(&self.schema)?;
        w.raw(&self.ruleset)?;
        w.option(self.membership_revision, |w, n| w.u64(n))?;
        w.raw(&self.membership_digest)?;
        Ok(w.finish())
    }
    pub(super) fn seal(mut self) -> Result<Self> {
        if self.generation == 0
            || self.term == 0
            || self.session == 0
            || self.incarnation == [0; 16]
            || !matches!(self.profile, 1 | 4 | 5)
            || self.membership_revision.is_none()
        {
            return Err(ManagedError::Corrupt);
        }
        self.bytes = self.encode_fields()?;
        Ok(self)
    }
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self> {
        Self {
            installation: r.fixed()?,
            generation: r.u64()?,
            account: AccountId::from_bytes(r.fixed()?),
            device: DeviceId::from_bytes(r.fixed()?),
            incarnation: r.fixed()?,
            term: r.u64()?,
            session: r.u64()?,
            profile: r.u32()?,
            schema: r.fixed()?,
            ruleset: r.fixed()?,
            membership_revision: r.option(Reader::u64)?,
            membership_digest: r.fixed()?,
            bytes: Vec::new(),
        }
        .seal()
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::plain(bytes, 197)?;
        let stamp = Self::read(&mut r)?;
        r.finish()?;
        same(bytes, stamp.canonical_bytes())?;
        Ok(stamp)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogFrontier {
    pub(super) revision: u64,
    pub(super) state_digest: [u8; 32],
    pub(super) head: [u8; 32],
}
impl CatalogFrontier {
    /// Reconstruct caller-retained protection data only. This does not attest custody or mint ownership.
    /// The actual catalog must independently match this protected revision/digest/head before any claim.
    pub fn from_parts(revision: u64, state_digest: [u8; 32], head: [u8; 32]) -> Result<Self> {
        if revision == 0 {
            return Err(ManagedError::StaleBackup);
        }
        Ok(Self {
            revision,
            state_digest,
            head,
        })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn state_digest(&self) -> [u8; 32] {
        self.state_digest
    }
    pub fn head(&self) -> [u8; 32] {
        self.head
    }
}
