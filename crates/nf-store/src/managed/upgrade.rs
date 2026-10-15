use super::{bytes::{Encoder, Reader, hash, next}, error::{ManagedError, Result}, foot::StorageFoot,
    schema, state::FullState};
use crate::{migration::{Composition, ProviderVersion, staging_record}, model::RequestStatus};
use nf_contract::identity::ProviderId;

pub(super) struct Descriptor {
    pub storage: u32,
    pub schema: [u8; 32],
    pub config: [u8; 32],
    pub content: [u8; 32],
    pub providers: Vec<ProviderVersion>,
    pub bytes: Vec<u8>,
}
impl Descriptor {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let decoded = staging_record::decode(bytes).map_err(|_| ManagedError::Corrupt)?;
        if staging_record::encode(&decoded) != bytes { return Err(ManagedError::Corrupt); }
        let mut r = Reader::plain(bytes, 1923)?;
        if r.take(b"NF-MIGRATION-COMPOSITION-1\0".len())? != b"NF-MIGRATION-COMPOSITION-1\0" {
            return Err(ManagedError::Corrupt);
        }
        let storage = r.u32()?; let schema = r.fixed()?; let config = r.fixed()?; let content = r.fixed()?;
        let n = r.u32()?; if n == 0 || n > 32 { return Err(ManagedError::Limit); }
        let mut providers = Vec::new();
        for _ in 0..n {
            providers.push(ProviderVersion { provider: ProviderId::from_bytes(r.fixed()?), implementation: r.fixed()?,
                version: r.u32()?, state_schema: r.u32()? });
        }
        r.finish()?;
        Ok(Self { storage, schema, config, content, providers, bytes: bytes.to_vec() })
    }
    pub fn of(composition: &Composition) -> Result<Self> { Self::read(&staging_record::encode(composition)) }
    pub fn digest(&self) -> [u8; 32] { hash(&self.bytes) }
    pub fn validate_registry(&self, world: &nf_kernel::World, supported: &[ProviderVersion]) -> Result<()> {
        let actual = world.to_spec();
        let mut support = supported.to_vec(); support.sort_unstable_by_key(|p| p.provider);
        if support != self.providers || actual.registry.len() != self.providers.len()
            || self.providers.iter().zip(&actual.registry).any(|(p, r)| p.provider != r.id
                || p.implementation != r.implementation_hash || p.version != r.version
                || p.state_schema != r.state_schema || p.version != 1 || p.state_schema != 1)
        { return Err(ManagedError::DerivationMismatch); }
        Ok(())
    }
}
pub(super) fn drained(state: &FullState) -> Result<()> {
    state.core.validate()?;
    if state.core.pending.is_some() || !state.core.reservations.is_empty() || state.policy.is_some()
        || state.core.requests.values().any(|r| matches!(r.status, RequestStatus::Pending { .. }))
    { return Err(ManagedError::NotDrained); }
    Ok(())
}
pub(super) fn validate(from: &Descriptor, to: &Descriptor, source: &FullState,
    supported: &[ProviderVersion]) -> Result<()> {
    drained(source)?;
    if from.storage != 1 || from.schema != crate::schema::schema_digest()
        || from.digest() != source.core.world.to_spec().ruleset_hash || to.storage != 4
        || to.schema != schema::active_schema_digest() || from.config == to.config
        || from.content != to.content || from.providers != to.providers
        || source.member.is_none() || !source.cancellations.is_empty()
    { return Err(ManagedError::DerivationMismatch); }
    from.validate_registry(&source.core.world, supported)?;
    to.validate_registry(&source.core.world, supported)
}
pub(super) fn source_binding(foot: &StorageFoot) -> Result<Vec<u8>> {
    if foot.profile != 1 || foot.schema != crate::schema::schema_digest() || foot.policy.is_some() {
        return Err(ManagedError::SourceMismatch);
    }
    let mut w = Encoder::new(b"NF-ACTIVE-SOURCE-1\0", 256)?;
    w.raw(foot.known.scope.universe.as_bytes())?; w.raw(foot.known.scope.history.as_bytes())?;
    w.u64(foot.known.store_revision)?; w.u64(foot.known.event_sequence.0)?;
    w.option(foot.known.membership_revision, |w, n| w.u64(n))?;
    w.raw(&foot.head)?; w.raw(&foot.core)?; w.option(foot.member, |w, sha| w.raw(&sha))?;
    w.u32(foot.quota)?; w.raw(&foot.rows)?; w.raw(&crate::schema::schema_digest())?;
    Ok(w.finish())
}
pub(super) fn initial_action(binding: &[u8], from: &Descriptor, to: &Descriptor) -> Result<Vec<u8>> {
    let mut w = Encoder::new(b"NF-ACTIVE-UPGRADE-1\0", 8192)?;
    w.blob(binding, 256)?; w.blob(&from.bytes, 1923)?; w.blob(&to.bytes, 1923)?; Ok(w.finish())
}
pub(super) fn lineage(binding: &[u8], action: &[u8], revision: u64,
    before: [u8; 32], after: [u8; 32]) -> Result<Vec<u8>> {
    let mut w = Encoder::new(b"NF-ACTIVE-LINEAGE-1\0", 16_384)?;
    w.blob(binding, 256)?; w.blob(action, 8192)?; w.u64(revision)?;
    w.raw(&before)?; w.raw(&after)?; Ok(w.finish())
}
pub(super) fn head(previous: [u8; 32], revision: u64, kind: u32, action: &[u8],
    before: [u8; 32], after: [u8; 32], lineage: [u8; 32]) -> Result<[u8; 32]> {
    if !(1..=7).contains(&kind) { return Err(ManagedError::Corrupt); }
    let mut w = Encoder::new(b"NF-ACTIVE-JOURNAL-1\0", 256)?;
    w.raw(&previous)?; w.u64(revision)?; w.u32(kind)?; w.raw(&hash(action))?;
    w.raw(&before)?; w.raw(&after)?; w.raw(&lineage)?; Ok(hash(&w.finish()))
}
pub(super) struct Initial {
    pub state: FullState,
    pub revision: u64,
    pub action: Vec<u8>,
    pub lineage: Vec<u8>,
    pub before: [u8; 32],
    pub head: [u8; 32],
}
pub(super) fn derive(source: &FullState, foot: &StorageFoot, from: &Descriptor,
    to: &Descriptor, supported: &[ProviderVersion]) -> Result<Initial> {
    validate(from, to, source, supported)?;
    let before = hash(&source.encode()?); if before != foot.state { return Err(ManagedError::SourceMismatch); }
    let mut state = source.clone();
    state.core.world = source.core.world.with_ruleset_hash(to.digest())?;
    let revision = next(foot.known.store_revision)?; state.validate(revision)?;
    let after = hash(&state.encode()?); let binding = source_binding(foot)?;
    let action = initial_action(&binding, from, to)?;
    let lineage = lineage(&binding, &action, revision, before, after)?;
    let head = head(foot.head, revision, 4, &action, before, after, hash(&lineage))?;
    Ok(Initial { state, revision, action, lineage, before, head })
}
