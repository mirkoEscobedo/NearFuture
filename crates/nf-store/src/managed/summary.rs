use super::{bytes::{Encoder, Reader, hash, same}, effect::Effect, error::{ManagedError, Result},
    foot::StorageFoot, pending::Pending, stamp::OwnerStamp};

#[derive(Clone)]
pub(super) struct Summary {
    pub sequence: u64,
    pub operation: [u8; 16],
    pub purpose: u32,
    pub stamp: OwnerStamp,
    pub artifact: [u8; 16],
    pub pre_pending: Option<[u8; 32]>,
    pub post_pending: Option<[u8; 32]>,
    pub stage: u32,
    pub pre: StorageFoot,
    pub post: StorageFoot,
    pub pre_state: [u8; 32],
    pub post_state: [u8; 32],
    pub action: [u8; 32],
    pub effect: Effect,
    pub intended: [u8; 32],
}
impl Summary {
    pub fn from_pending(pending: &Pending) -> Result<Self> {
        let pre_pending = if pending.stage == 2 {
            let mut first = pending.clone(); first.stage = 1; Some(first.digest()?)
        } else { None };
        Ok(Self { sequence: pending.sequence, operation: pending.operation, purpose: pending.purpose,
            stamp: pending.stamp.clone(), artifact: pending.artifact, pre_pending,
            post_pending: Some(pending.digest()?), stage: pending.stage,
            pre: pending.pre.clone(), post: pending.post.clone(), pre_state: hash(&pending.pre_state),
            post_state: hash(&pending.post_state), action: hash(&pending.action), effect: pending.effect.clone(),
            intended: pending.intended.digest()? })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.sequence == 0 || !matches!(self.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
            || !(1..=2).contains(&self.stage) || (self.stage == 2 && self.purpose != 8)
            || self.post_pending.is_none() || self.pre_state != self.pre.state || self.post_state != self.post.state
            || (self.stage == 1 && self.pre_pending.is_some()) || (self.stage == 2 && self.pre_pending.is_none())
        { return Err(ManagedError::Corrupt); }
        self.effect.check_feet(&self.pre, &self.post)?;
        let mut w = Encoder::new(b"NF-OWNER-PENDING-SUMMARY-1\0", 16_384)?;
        w.u64(self.sequence)?; w.raw(&self.operation)?; w.u32(self.purpose)?;
        w.raw(self.stamp.canonical_bytes())?; w.raw(&self.artifact)?;
        w.option(self.pre_pending, |w, sha| w.raw(&sha))?;
        w.option(self.post_pending, |w, sha| w.raw(&sha))?; w.u32(self.stage)?;
        w.blob(self.pre.canonical_bytes(), 2048)?; w.blob(self.post.canonical_bytes(), 2048)?;
        w.raw(&self.pre_state)?; w.raw(&self.post_state)?; w.raw(&self.action)?;
        w.blob(&self.effect.encode()?, 8192)?; w.raw(&self.intended)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-PENDING-SUMMARY-1\0", 16_384)?;
        let value = Self { sequence: r.u64()?, operation: r.fixed()?, purpose: r.u32()?, stamp: OwnerStamp::read(&mut r)?,
            artifact: r.fixed()?, pre_pending: r.option(Reader::fixed)?, post_pending: r.option(Reader::fixed)?, stage: r.u32()?,
            pre: StorageFoot::decode(r.blob(2048)?)?, post: StorageFoot::decode(r.blob(2048)?)?,
            pre_state: r.fixed()?, post_state: r.fixed()?, action: r.fixed()?, effect: Effect::decode(r.blob(8192)?)?, intended: r.fixed()? };
        r.finish()?; same(bytes, &value.encode()?)?; Ok(value)
    }
}
