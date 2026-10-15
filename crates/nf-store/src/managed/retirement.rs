use super::{bytes::{Encoder, Reader, same}, error::{ManagedError, Result}, foot::StorageFoot,
    stamp::OwnerStamp, upgrade::{Descriptor, source_binding}, schema};

#[derive(Clone)]
pub(super) struct Retirement {
    pub installation: [u8; 16],
    pub stamp: OwnerStamp,
    pub sequence: u64,
    pub operation: [u8; 16],
    pub source: [u8; 16],
    pub retained: [u8; 16],
    pub target: [u8; 16],
    pub binding: Vec<u8>,
    pub source_foot: StorageFoot,
    pub target_foot: StorageFoot,
    pub composition: Vec<u8>,
    pub lineage: [u8; 32],
    pub old_binary: [u8; 32],
    pub new_binary: [u8; 32],
}
impl Retirement {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let to = Descriptor::read(&self.composition)?;
        if self.installation != self.stamp.installation || self.stamp.profile != 1 || self.sequence == 0
            || self.source == self.retained || self.source == self.target || self.retained == self.target
            || self.source_foot.profile != 1 || self.target_foot.profile != 4
            || self.source_foot.schema != crate::schema::schema_digest()
            || self.target_foot.schema != schema::active_schema_digest()
            || self.source_foot.known.scope != self.target_foot.known.scope
            || self.source_foot.known.store_revision.checked_add(1) != Some(self.target_foot.known.store_revision)
            || self.source_foot.known.event_sequence != self.target_foot.known.event_sequence
            || self.source_foot.known.membership_revision != self.target_foot.known.membership_revision
            || self.source_foot.member != self.target_foot.member || self.source_foot.quota != self.target_foot.quota
            || self.source_foot.policy.is_some() || self.target_foot.policy.is_some()
            || self.target_foot.lineage != Some(self.lineage) || source_binding(&self.source_foot)? != self.binding
            || to.storage != 4 || to.schema != schema::active_schema_digest()
        { return Err(ManagedError::DerivationMismatch); }
        let mut w = Encoder::new(b"NF-OWNER-RETIRE-1\0", 8192)?;
        w.raw(&self.installation)?; w.raw(self.stamp.canonical_bytes())?;
        w.u64(self.sequence)?; w.raw(&self.operation)?;
        w.raw(&self.source)?; w.raw(&self.retained)?; w.raw(&self.target)?;
        w.blob(&self.binding, 256)?; w.blob(self.source_foot.canonical_bytes(), 2048)?;
        w.blob(self.target_foot.canonical_bytes(), 2048)?; w.blob(&self.composition, 1923)?;
        w.raw(&self.lineage)?; w.raw(&self.old_binary)?; w.raw(&self.new_binary)?;
        w.raw(&schema::retired_schema_digest())?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-RETIRE-1\0", 8192)?;
        let value = Self { installation: r.fixed()?, stamp: OwnerStamp::read(&mut r)?, sequence: r.u64()?,
            operation: r.fixed()?, source: r.fixed()?, retained: r.fixed()?, target: r.fixed()?,
            binding: r.blob(256)?.to_vec(), source_foot: StorageFoot::decode(r.blob(2048)?)?,
            target_foot: StorageFoot::decode(r.blob(2048)?)?, composition: r.blob(1923)?.to_vec(),
            lineage: r.fixed()?, old_binary: r.fixed()?, new_binary: r.fixed()? };
        if r.fixed::<32>()? != schema::retired_schema_digest() { return Err(ManagedError::Corrupt); }
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
}
