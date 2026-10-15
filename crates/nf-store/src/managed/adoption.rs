use super::{admission::{Input as Admission, Operation}, book::{self, Book}, bytes::hash,
    catalog::{Catalog, CreateFailure}, catalog_action::Action, catalog_payload::Payload,
    catalog_state::CatalogState, challenge::{Binding, Challenge}, delta::Delta,
    engine::Engine, error::{ManagedError, Result}, failure::{AdoptionFailure, AdoptionOutcome},
    file_io, foot::StorageFoot, input::AdoptionInput, maps::Maps, prospective::Prospective,
    reconcile::ReconciliationHandle, registry::{Manifest, RegistryRow}, source_owner::SourceOwner,
    source_plan, stamp::OwnerStamp, state::FullState, upgrade::{self, Descriptor}, world_file::WorldFile};
use crate::{KnownFrontiers, Store};
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

struct DraftData {
    catalog: PathBuf,
    source_path: PathBuf,
    minimum: KnownFrontiers,
    state: FullState,
    foot: StorageFoot,
    installation: [u8; 16],
    source_id: [u8; 16],
    from: Descriptor,
    supported: Vec<crate::migration::ProviderVersion>,
    delta: Delta,
}
pub struct AdoptionDraft { source: Store, data: DraftData, book: Book }
pub(super) fn begin(source: Store, catalog: PathBuf, input: AdoptionInput<'_>)
    -> std::result::Result<AdoptionDraft, AdoptionFailure> {
    let checked = prepare(&source, catalog, &input).and_then(|data| Ok((data, Book::new()?)));
    match checked {
        Ok((data, book)) => Ok(AdoptionDraft { source, data, book }),
        Err(error) => Err(AdoptionFailure { error, source: Some(source), reconciliation: None }),
    }
}
fn prepare(source: &Store, catalog: PathBuf, input: &AdoptionInput<'_>) -> Result<DraftData> {
    super::schema::local_absolute(&catalog)?;
    super::schema::absent(&catalog)?; super::schema::companions_absent(&catalog, false)?;
    super::world_file::bind_path(&source.connection, &input.source_path)?;
    super::schema::header(&input.source_path, super::schema::Profile::Source)?;
    super::journal::cold(&source.connection, &input.source_path)?;
    source.verified_source_head(input.minimum)?;
    let member = crate::identity::load(&source.connection, input.minimum.scope)?.ok_or(ManagedError::Unauthorized)?;
    let quota: u32 = source.connection.pragma_query_value(None, "max_page_count", |r| r.get(0))?;
    let (state, foot) = source_plan::inspect(source, member, quota, None, None)?;
    upgrade::drained(&state)?; super::world_file::quota_check(&source.connection, quota)?;
    if input.artifact_provenance.len() > 2048 || input.artifact_provenance.as_bytes().contains(&0)
        || input.binary_evidence.qualification.len() > 2048
        || input.binary_evidence.qualification.as_bytes().contains(&0) || input.supported.len() > 32
    { return Err(ManagedError::Limit); }
    let from = Descriptor::of(input.composition)?;
    let binary_from = Descriptor::of(input.binary_evidence.from)?;
    let binary_to = Descriptor::of(input.binary_evidence.to)?;
    if from.bytes != binary_from.bytes { return Err(ManagedError::UnsupportedFormat); }
    upgrade::validate(&from, &binary_to, &state, input.supported)?;
    let paths = [&catalog, &input.source_path, &input.binary_evidence.old.path, &input.binary_evidence.new.path];
    for (i, path) in paths.iter().enumerate() {
        super::schema::local_absolute(path)?;
        if paths[..i].contains(path) { return Err(ManagedError::SourceMismatch); }
    }
    file_io::verify(&input.binary_evidence.old)?; file_io::verify(&input.binary_evidence.new)?;
    let source_pin = file_io::observe(&input.source_path, input.artifact_provenance)?;
    let installation = book::random()?; let source_id = book::random()?;
    let old_id = book::random()?; let new_id = book::random()?;
    if source_id == old_id || source_id == new_id || old_id == new_id { return Err(ManagedError::Limit); }
    let scope = state.core.scope();
    let rows = vec![
        artifact(installation, scope, 1, source_id, &source_pin, 1, foot.schema, from.bytes.clone())?,
        artifact(installation, scope, 4, old_id, &input.binary_evidence.old, 0, [0; 32], binary_from.bytes)?,
        artifact(installation, scope, 5, new_id, &input.binary_evidence.new, 0, [0; 32], binary_to.bytes)?,
    ];
    let mut supported = input.supported.to_vec(); supported.sort_unstable_by_key(|p| p.provider);
    let delta = Delta::additions(rows)?;
    delta.apply(&Maps::default(), super::delta::RegistryPurpose::Adopt, None)?;
    Ok(DraftData { catalog, source_path: input.source_path.clone(), minimum: input.minimum,
        state, foot, installation, source_id, from, supported, delta })
}
pub(super) fn artifact(installation: [u8; 16], scope: nf_identity::model::Scope, kind: u32,
    id: [u8; 16], pin: &super::input::FilePin, profile: u32, schema: [u8; 32], composition: Vec<u8>) -> Result<RegistryRow> {
    RegistryRow::from_manifest(&Manifest { installation, scope, kind, id, path: pin.path.clone(), bytes: pin.bytes,
        digest: pin.digest, profile, schema, provenance: pin.provenance.clone(), composition })
}
impl AdoptionDraft {
    fn verify_source(&mut self) -> Result<()> {
        let result = self.verify_source_inner();
        if result.is_err() { self.book.clear(); }
        result
    }
    fn verify_source_inner(&self) -> Result<()> {
        self.source.verified_source_head(self.data.minimum)?;
        let member = self.data.state.member.as_ref().ok_or(ManagedError::Unauthorized)?.clone();
        let (state, foot) = source_plan::inspect(&self.source, member, self.data.state.quota, None, Some(&self.data.foot))?;
        if state != self.data.state || foot != self.data.foot { return Err(ManagedError::SourceMismatch); }
        super::journal::cold(&self.source.connection, &self.data.source_path)?;
        super::schema::absent(&self.data.catalog)?; super::schema::companions_absent(&self.data.catalog, false)?;
        for (_, row) in &self.data.delta.registry {
            let row = row.1.as_ref().ok_or(ManagedError::Corrupt)?;
            let manifest = Manifest::decode(&row.manifest)?;
            file_io::verify(&super::input::FilePin::new(row.path.clone(), row.bytes, row.digest, &manifest.provenance)?)?;
        }
        Ok(())
    }
    fn admission(&self) -> Result<Vec<u8>> {
        Admission { installation: self.data.installation, scope: self.data.foot.known.scope,
            foot: self.data.foot.clone(), composition: self.data.from.bytes.clone(), catalog: self.data.catalog.clone(),
            delta: self.data.delta.clone(), operation: Operation::Adopt { source: self.data.source_id,
                supported: self.data.supported.clone() } }.encode()
    }
    pub fn issue_adoption_challenge(&mut self, public: &PublicIdentity, cutoff_ms: u64) -> Result<Challenge> {
        self.verify_source()?;
        book::eligible(self.data.state.member.as_ref().ok_or(ManagedError::Unauthorized)?, public, true)?;
        let times = self.book.times(cutoff_ms, 0)?;
        let candidate = OwnerStamp { installation: self.data.installation, generation: 1,
            account: public.account, device: public.device, incarnation: book::random()?, term: 1, session: 1,
            profile: 1, schema: self.data.foot.schema, ruleset: self.data.from.digest(),
            membership_revision: self.data.foot.known.membership_revision,
            membership_digest: self.data.foot.member.ok_or(ManagedError::Unauthorized)?, bytes: Vec::new() }.seal()?;
        let admission = self.admission()?; let input = hash(&admission);
        let prospective = Prospective { purpose: 9, installation: self.data.installation,
            scope: self.data.foot.known.scope, catalog: None, candidate: candidate.clone(), foot: self.data.foot.clone(),
            peer: public.peer.clone(), input, times: times.clone() };
        self.book.issue(Binding::Prospective { purpose: 9, candidate, input }, &times,
            prospective.encode()?, admission, Vec::new(), Vec::new())
    }
    pub fn finish_adoption(mut self, proof: DeviceProof) -> std::result::Result<SourceOwner, AdoptionOutcome> {
        let verified = (|| {
            self.verify_source()?;
            let member = self.data.state.member.as_ref().ok_or(ManagedError::Unauthorized)?;
            let (binding, transcript, _) = self.book.consume(&proof, member, ProtectedOperation::Administration, true)?;
            let candidate = match binding {
                Binding::Prospective { purpose: 9, candidate, input } if input == hash(&self.admission()?) => candidate,
                _ => return Err(ManagedError::OperationConflict),
            };
            let prospective = Prospective::decode(&transcript)?; let utc = book::now_ms()?;
            if prospective.candidate != candidate || utc < prospective.times.issued || utc >= prospective.times.expires {
                return Err(ManagedError::Expired);
            }
            let maps = self.data.delta.apply(&Maps::default(), super::delta::RegistryPurpose::Adopt, None)?;
            let (registry, receipts) = maps.digests()?;
            let state = CatalogState { installation: self.data.installation, scope: self.data.foot.known.scope,
                phase: 2, generation: 1, term: 1, session: 1, owner: Some(super::catalog_rules::identity(&candidate)),
                consumed: 0, selected: Some(self.data.source_id), source: self.data.source_id,
                current: Some(self.data.foot.clone()), original: self.data.foot.clone(), target: None,
                retirement: None, pending: None, source_policy: None, utc, registry, receipts };
            let action = Action { payload: Payload::Owner { adopt: true, transcript, proof,
                stamp: candidate.clone(), foot: self.data.foot.clone() }, delta: self.data.delta.clone() };
            Ok((candidate, state, action))
        })();
        let (stamp, state, action) = match verified {
            Ok(value) => value, Err(error) => return Err(self.before_failure(error)),
        };
        let catalog = match Catalog::create(self.data.catalog.clone(), state, action) {
            Ok(value) => value, Err(CreateFailure::Before(error)) => return Err(self.before_failure(error)),
            Err(CreateFailure::After(error)) => return Err(self.after_failure(error)),
        };
        let recovery = ReconciliationHandle::from_catalog(&catalog, None);
        let world = match WorldFile::consume_source(self.source, self.data.source_path, self.data.minimum) {
            Ok(value) => value,
            Err((error, _source)) => return Err(AdoptionFailure { error, source: None, reconciliation: Some(recovery) }),
        };
        let owner = Engine::new(catalog, Some(world), Some(stamp)).and_then(SourceOwner::from_engine);
        owner.map_err(|error| AdoptionFailure { error, source: None, reconciliation: Some(recovery) })
    }
    fn before_failure(self, error: ManagedError) -> AdoptionFailure {
        AdoptionFailure { error, source: Some(self.source), reconciliation: None }
    }
    fn after_failure(self, error: ManagedError) -> AdoptionFailure {
        let recovery = ReconciliationHandle::for_adoption(self.data.catalog, self.data.installation);
        AdoptionFailure { error, source: None, reconciliation: Some(recovery) }
    }
}
