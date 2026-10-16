use super::{
    auth,
    bytes::{Encoder, Reader, hash, same},
    catalog_state::CatalogState,
    effect::Effect,
    error::{ManagedError, Result},
    foot::StorageFoot,
    stamp::{CatalogFrontier, OwnerStamp},
    state::FullState,
};

#[derive(Clone)]
pub(super) struct Pending {
    pub sequence: u64,
    pub operation: [u8; 16],
    pub purpose: u32,
    pub stamp: OwnerStamp,
    pub before: CatalogFrontier,
    pub artifact: [u8; 16],
    pub pre: StorageFoot,
    pub post: StorageFoot,
    pub pre_state: Vec<u8>,
    pub post_state: Vec<u8>,
    pub action: Vec<u8>,
    pub intended: CatalogState,
    pub effect: Effect,
    pub stage: u32,
}
fn verify_state(bytes: &[u8], foot: &StorageFoot) -> Result<()> {
    let state = FullState::decode(bytes, foot.known.store_revision)?;
    let member = state
        .member
        .as_ref()
        .map(nf_identity::codec::encode_state)
        .transpose()?;
    if state.core.scope() != foot.known.scope
        || state.core.world.view().event_seq() != foot.known.event_sequence
        || state.member.as_ref().map(|m| m.revision) != foot.known.membership_revision
        || hash(&crate::codec::encode(&state.core)?) != foot.core
        || member.as_deref().map(hash) != foot.member
        || hash(bytes) != foot.state
        || state.quota != foot.quota
        || state.policy != foot.policy
        || state.cancellation_digest()? != foot.cancellations
    {
        return Err(ManagedError::DerivationMismatch);
    }
    Ok(())
}
impl Pending {
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0
            || !matches!(self.purpose, 1 | 2 | 3 | 5 | 6 | 7 | 8)
            || !(1..=2).contains(&self.stage)
            || (self.stage == 2 && self.purpose != 8)
            || self.pre.known.scope != self.post.known.scope
            || self.stamp.installation != self.intended.installation
            || self.stamp.profile != self.pre.profile
            || self.intended.consumed != self.sequence
            || self.intended.current.as_ref() != Some(&self.post)
            || self.intended.pending.is_some()
        {
            return Err(ManagedError::Corrupt);
        }
        verify_state(&self.pre_state, &self.pre)?;
        verify_state(&self.post_state, &self.post)?;
        let (auth, _, _) = auth::decode_action(&self.action)?;
        if auth.stamp != self.stamp
            || auth.purpose != self.purpose
            || auth.sequence != self.sequence
            || auth.operation != self.operation
            || auth.revision != self.pre.known.store_revision
            || auth.state != self.pre.state
            || auth.head != self.pre.head
        {
            return Err(ManagedError::Authentication);
        }
        self.effect.encode()?;
        self.effect.check_feet(&self.pre, &self.post)?;
        self.intended.encode()?;
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut w = Encoder::new(b"NF-OWNER-PENDING-1\0", 5_242_880)?;
        w.u64(self.sequence)?;
        w.raw(&self.operation)?;
        w.u32(self.purpose)?;
        w.raw(self.stamp.canonical_bytes())?;
        w.u64(self.before.revision)?;
        w.raw(&self.before.state_digest)?;
        w.raw(&self.before.head)?;
        w.raw(&self.artifact)?;
        w.blob(self.pre.canonical_bytes(), 2048)?;
        w.blob(self.post.canonical_bytes(), 2048)?;
        w.blob(&self.pre_state, 1_884_160)?;
        w.blob(&self.post_state, 1_884_160)?;
        w.blob(&self.action, 1_114_112)?;
        w.blob(&self.intended.encode()?, 16_384)?;
        w.blob(&self.effect.encode()?, 8192)?;
        w.u32(self.stage)?;
        Ok(w.finish())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes, b"NF-OWNER-PENDING-1\0", 5_242_880)?;
        let value = Self {
            sequence: r.u64()?,
            operation: r.fixed()?,
            purpose: r.u32()?,
            stamp: OwnerStamp::read(&mut r)?,
            before: CatalogFrontier {
                revision: r.u64()?,
                state_digest: r.fixed()?,
                head: r.fixed()?,
            },
            artifact: r.fixed()?,
            pre: StorageFoot::decode(r.blob(2048)?)?,
            post: StorageFoot::decode(r.blob(2048)?)?,
            pre_state: r.blob(1_884_160)?.to_vec(),
            post_state: r.blob(1_884_160)?.to_vec(),
            action: r.blob(1_114_112)?.to_vec(),
            intended: CatalogState::decode(r.blob(16_384)?)?,
            effect: Effect::decode(r.blob(8192)?)?,
            stage: r.u32()?,
        };
        r.finish()?;
        same(bytes, &value.encode()?)?;
        Ok(value)
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        Ok(hash(&self.encode()?))
    }
}
