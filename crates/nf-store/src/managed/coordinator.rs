use super::{admission::{Input, Operation}, adoption::{self, AdoptionDraft}, book, bytes::{hash, next},
    catalog::{Catalog, PendingChange}, catalog_action::Action, catalog_payload::Payload, catalog_rules,
    challenge::{Binding, Challenge}, delta::Delta, engine::Engine, error::{ManagedError, Result},
    failure::AdoptionFailure, foot::StorageFoot, input::AdoptionInput, owner::CurrentOwner,
    prospective::Prospective, sealed::ManagedDrainedSource,
    selection::SourceFenced, stamp::{CatalogFrontier, OwnerStamp}, world_file::WorldFile};
use crate::Store;
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

pub struct LocalCoordinator;
pub struct UnclaimedCoordinator { pub(super) engine: Engine }
impl LocalCoordinator {
    pub fn begin_adoption(source: Store, path: PathBuf, input: AdoptionInput<'_>)
        -> std::result::Result<AdoptionDraft, AdoptionFailure> { adoption::begin(source, path, input) }
    pub fn open_existing(path: PathBuf, installation: [u8; 16], protected: CatalogFrontier)
        -> Result<UnclaimedCoordinator> {
        if protected.revision == 0 { return Err(ManagedError::StaleBackup); }
        let catalog = Catalog::open(path, installation, protected)?;
        let world = if catalog.pending.is_none() {
            let foot = catalog.state.current.as_ref().ok_or(ManagedError::Corrupt)?;
            let kind = if foot.profile == 4 { 3 } else { 1 };
            let row = catalog.maps.registry.get(&kind).ok_or(ManagedError::Corrupt)?;
            Some(WorldFile::open_owned(row.path.clone(), foot, catalog.state.retirement)?)
        } else { None };
        let engine = Engine::new(catalog, world, None)?;
        Ok(UnclaimedCoordinator { engine })
    }
}
impl UnclaimedCoordinator {
    pub fn installation(&self) -> [u8; 16] { self.engine.catalog.state.installation }
    pub fn catalog_frontier(&self) -> CatalogFrontier { self.engine.catalog.frontier }
    fn verify(&mut self) -> Result<()> {
        self.engine.guard(Self::verify_engine)?;
        if !matches!(self.engine.catalog.state.phase, 1 | 2 | 3 | 6 | 7) {
            return Err(ManagedError::OwnerMismatch);
        }
        Ok(())
    }
    fn verify_engine(engine: &Engine) -> Result<()> {
        engine.catalog.verify_cached()?;
        if engine.catalog.pending.is_some() { return Err(ManagedError::ReconciliationRequired); }
        let foot = engine.catalog.state.current.as_ref().ok_or(ManagedError::Corrupt)?;
        engine.world()?.verify(foot, engine.catalog.state.retirement)?;
        if engine.catalog.state.phase == 8 { return Err(ManagedError::Quarantined); }
        engine.verify_binary(if foot.profile == 4 { 5 } else { 4 })
    }
    pub fn issue_owner_challenge(&mut self, public: &PublicIdentity, exact: &[u8], cutoff: u64)
        -> Result<Challenge> {
        self.verify()?;
        let state = &self.engine.catalog.state; let foot = self.engine.world()?.foot.clone();
        if foot.canonical_bytes() != exact { return Err(ManagedError::SourceMismatch); }
        book::eligible(self.engine.member()?, public, true)?;
        let candidate = OwnerStamp { installation: state.installation, generation: state.generation,
            account: public.account, device: public.device, incarnation: book::random()?, term: next(state.term)?,
            session: next(state.session)?, profile: foot.profile, schema: foot.schema,
            ruleset: self.engine.world()?.state.core.world.to_spec().ruleset_hash,
            membership_revision: foot.known.membership_revision,
            membership_digest: foot.member.ok_or(ManagedError::Unauthorized)?, bytes: Vec::new() }.seal()?;
        let input = self.claim_input(&candidate, foot.clone())?.encode()?;
        let input_hash = hash(&input); let times = self.engine.book.times(cutoff, self.engine.catalog.state.utc)?;
        let prospective = Prospective { purpose: 10, installation: candidate.installation,
            scope: foot.known.scope, catalog: Some(self.engine.catalog.frontier), candidate: candidate.clone(),
            foot, peer: public.peer.clone(), input: input_hash, times: times.clone() };
        self.engine.book.issue(Binding::Prospective { purpose: 10, candidate, input: input_hash },
            &times, prospective.encode()?, input, Vec::new(), Vec::new())
    }
    fn claim_input(&self, candidate: &OwnerStamp, foot: StorageFoot) -> Result<Input> {
        Ok(Input { installation: self.installation(), scope: foot.known.scope, foot,
            composition: Vec::new(), catalog: self.engine.catalog.path.clone(), delta: Delta::default(),
            operation: Operation::Claim { generation: candidate.generation, term: candidate.term,
                session: candidate.session, incarnation: candidate.incarnation } })
    }
    fn claim_engine(mut self, proof: DeviceProof, sealed: bool) -> Result<Engine> {
        self.verify()?;
        if (self.engine.catalog.state.phase == 3) != sealed { return Err(ManagedError::OwnerMismatch); }
        let member = self.engine.member()?.clone();
        let (binding, transcript, _) = self.engine.book.consume(&proof, &member,
            ProtectedOperation::Administration, true)?;
        let (candidate, input) = match binding { Binding::Prospective { purpose: 10, candidate, input } =>
            (candidate, input), _ => return Err(ManagedError::OperationConflict) };
        let foot = self.engine.world()?.foot.clone();
        let prospective = Prospective::decode(&transcript)?;
        let utc = book::now_ms()?;
        if input != hash(&self.claim_input(&candidate, foot.clone())?.encode()?)
            || prospective.catalog != Some(self.engine.catalog.frontier)
            || prospective.input != input || prospective.candidate != candidate
            || prospective.foot != foot || utc < prospective.times.issued || utc >= prospective.times.expires
        { return Err(ManagedError::StaleSession); }
        catalog_rules::claim_stamp(&self.engine.catalog.state, &candidate, &foot)?;
        let mut after = self.engine.catalog.state.clone(); after.utc = utc;
        after.term = candidate.term; after.session = candidate.session;
        after.owner = Some(catalog_rules::identity(&candidate));
        after.phase = if sealed { 3 } else if foot.profile == 1 { 2 } else { 7 };
        let result = self.engine.catalog.append(Action { payload: Payload::Owner { adopt: false,
            transcript, proof, stamp: candidate.clone(), foot }, delta: Delta::default() }, after, PendingChange::Keep);
        if let Err(error) = result { self.engine.stop(); return Err(error); }
        self.engine.book.clear(); self.engine.stamp = Some(candidate); Ok(self.engine)
    }
    pub fn claim(self, proof: DeviceProof) -> Result<CurrentOwner> {
        CurrentOwner::from_engine(self.claim_engine(proof, false)?)
    }
    /// Same purpose10 proof route, yielding only sealed management at phase3.
    pub fn resume_sealed(self, proof: DeviceProof) -> Result<ManagedDrainedSource> {
        ManagedDrainedSource::from_engine(self.claim_engine(proof, true)?)
    }
    pub fn resume_fenced(mut self) -> Result<SourceFenced> {
        self.engine.guard(|engine| {
            engine.catalog.verify_cached()?;
            if engine.catalog.state.phase != 5 || engine.catalog.pending.is_some()
                || engine.catalog.state.owner.is_some() { return Err(ManagedError::OwnerMismatch); }
            engine.world()?.verify(engine.catalog.state.current.as_ref().ok_or(ManagedError::Corrupt)?,
                engine.catalog.state.retirement)
        })?;
        let world = self.engine.world.take().ok_or(ManagedError::ReconciliationRequired)?;
        if let Err(error) = world.close() { self.engine.stop(); return Err(error); }
        SourceFenced::from_engine(self.engine)
    }
    pub fn into_reconciliation(self) -> super::reconcile::ReconciliationHandle {
        super::reconcile::ReconciliationHandle::from_engine(self.engine)
    }
}
