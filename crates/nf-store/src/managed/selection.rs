use super::{admission::{Input, Operation}, book, bytes::{hash, next}, catalog::PendingChange,
    catalog_action::Action, catalog_payload::Payload, challenge::{Binding, Challenge},
    coordinator::UnclaimedCoordinator, delta::Delta, engine::Engine, error::{ManagedError, Result},
    file_io, prospective::Prospective, registry::Manifest, retired_read, stamp::{CatalogFrontier, OwnerStamp},
    world_file::WorldFile};
use crate::KnownFrontiers;
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

pub struct SourceFenced { pub(super) engine: Engine }
impl SourceFenced {
    pub(super) fn from_engine(mut engine: Engine) -> Result<Self> {
        engine.guard(|engine| engine.catalog.verify_cached())?;
        if engine.catalog.state.phase != 5 || engine.catalog.state.owner.is_some()
            || engine.catalog.state.pending.is_some() || engine.world.is_some() {
            return Err(ManagedError::ReconciliationRequired);
        }
        Ok(Self { engine })
    }
    pub fn installation(&self) -> [u8; 16] { self.engine.catalog.state.installation }
    pub fn catalog_frontier(&self) -> CatalogFrontier { self.engine.catalog.frontier }
    pub(super) fn verify(&mut self) -> Result<WorldFile> { self.engine.guard(Self::verify_engine) }
    fn verify_engine(engine: &Engine) -> Result<WorldFile> {
        engine.catalog.verify_cached()?;
        let state = &engine.catalog.state;
        if state.phase != 5 || state.pending.is_some() || state.owner.is_some() { return Err(ManagedError::OwnerMismatch); }
        let source_foot = state.current.as_ref().ok_or(ManagedError::Corrupt)?;
        let target_foot = state.target.as_ref().ok_or(ManagedError::Corrupt)?;
        let source = engine.catalog.maps.registry.get(&1).ok_or(ManagedError::Corrupt)?;
        let target = engine.catalog.maps.registry.get(&3).ok_or(ManagedError::Corrupt)?;
        let retained = engine.catalog.maps.registry.get(&2).ok_or(ManagedError::Corrupt)?;
        engine.verify_binary(4)?; engine.verify_binary(5)?;
        let retired = WorldFile::open_owned(source.path.clone(), source_foot, state.retirement)?;
        let (_, record) = retired_read::inspect(&retired.connection, source_foot, state.retirement)?;
        if record.installation != state.installation || record.source != state.source
            || record.retained != retained.artifact || record.target != target.artifact
            || record.target_foot != *target_foot
            || record.old_binary != engine.catalog.maps.registry.get(&4).ok_or(ManagedError::Corrupt)?.digest
            || record.new_binary != engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.digest {
            return Err(ManagedError::TargetMismatch);
        }
        // Independent read/close/fsync of the already retired source precedes every selection admission.
        retired.close()?;
        let manifest = Manifest::decode(&retained.manifest)?;
        file_io::verify(&super::input::FilePin::new(retained.path.clone(), retained.bytes, retained.digest, &manifest.provenance)?)?;
        super::readonly::verify(&retained.path, &record.source_foot, None)?;
        let manifest = Manifest::decode(&target.manifest)?;
        file_io::verify(&super::input::FilePin::new(target.path.clone(), target.bytes, target.digest, &manifest.provenance)?)?;
        let binary = Manifest::decode(&engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.manifest)?;
        if manifest.composition != record.composition || binary.composition != record.composition {
            return Err(ManagedError::UnsupportedFormat);
        }
        WorldFile::open(target.path.clone(), target_foot, false, None)
    }
    fn input(&self, candidate: &OwnerStamp) -> Result<Input> {
        let state = &self.engine.catalog.state;
        let target = self.engine.catalog.maps.registry.get(&3).ok_or(ManagedError::Corrupt)?;
        let binary = self.engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?;
        Ok(Input { installation: state.installation, scope: state.scope,
            foot: state.current.clone().ok_or(ManagedError::Corrupt)?,
            composition: Manifest::decode(&target.manifest)?.composition, catalog: self.engine.catalog.path.clone(),
            delta: Delta::default(), operation: Operation::Select { source: state.source, target: target.artifact,
                retired: state.current.clone().ok_or(ManagedError::Corrupt)?,
                initial: state.target.clone().ok_or(ManagedError::Corrupt)?,
                retirement: state.retirement.ok_or(ManagedError::Corrupt)?, binary: binary.manifest_digest(),
                generation: candidate.generation, term: candidate.term, session: candidate.session,
                incarnation: candidate.incarnation } })
    }
    pub fn issue_selection_challenge(&mut self, path: PathBuf, ruleset: [u8; 32], minimum: KnownFrontiers,
        public: &PublicIdentity, cutoff: u64) -> Result<Challenge> {
        let target = self.verify()?;
        if target.path != path || target.state.core.world.to_spec().ruleset_hash != ruleset
            || minimum.scope != target.foot.known.scope || minimum.store_revision > target.foot.known.store_revision
            || minimum.event_sequence > target.foot.known.event_sequence
            || minimum.membership_revision > target.foot.known.membership_revision { return Err(ManagedError::StaleBackup); }
        book::eligible(target.state.member.as_ref().ok_or(ManagedError::Unauthorized)?, public, true)?;
        let state = &self.engine.catalog.state;
        let candidate = OwnerStamp { installation: state.installation, generation: next(state.generation)?,
            account: public.account, device: public.device, incarnation: book::random()?, term: next(state.term)?,
            session: next(state.session)?, profile: 4, schema: target.foot.schema, ruleset,
            membership_revision: target.foot.known.membership_revision,
            membership_digest: target.foot.member.ok_or(ManagedError::Unauthorized)?, bytes: Vec::new() }.seal()?;
        if let Err(error) = target.close() { self.engine.stop(); return Err(error); }
        let input = self.input(&candidate)?.encode()?; let input_hash = hash(&input);
        let times = self.engine.book.times(cutoff, self.engine.catalog.state.utc)?;
        let prospective = Prospective { purpose: 13, installation: self.installation(),
            scope: self.engine.catalog.state.scope, catalog: Some(self.engine.catalog.frontier), candidate: candidate.clone(),
            foot: self.engine.catalog.state.current.clone().ok_or(ManagedError::Corrupt)?, peer: public.peer.clone(),
            input: input_hash, times: times.clone() };
        self.engine.book.issue(Binding::Prospective { purpose: 13, candidate, input: input_hash }, &times,
            prospective.encode()?, input, Vec::new(), Vec::new())
    }
    pub fn select_target(mut self, proof: DeviceProof) -> Result<UnclaimedCoordinator> {
        let target = self.verify()?;
        let member = target.state.member.as_ref().ok_or(ManagedError::Unauthorized)?;
        let (binding, transcript, _) = self.engine.book.consume(&proof, member, ProtectedOperation::Administration, true)?;
        let (candidate, input) = match binding { Binding::Prospective { purpose: 13, candidate, input } => (candidate, input),
            _ => return Err(ManagedError::OperationConflict) };
        let auth = Prospective::decode(&transcript)?; let utc = book::now_ms()?;
        if auth.catalog != Some(self.engine.catalog.frontier) || input != hash(&self.input(&candidate)?.encode()?)
            || auth.input != input || auth.candidate != candidate || utc < auth.times.issued || utc >= auth.times.expires {
            return Err(ManagedError::StaleSession);
        }
        let mut after = self.engine.catalog.state.clone(); after.phase = 6; after.owner = None;
        after.selected = Some(self.engine.catalog.maps.registry.get(&3).ok_or(ManagedError::Corrupt)?.artifact);
        after.generation = candidate.generation; after.current = Some(target.foot.clone()); after.source_policy = None; after.utc = utc;
        let payload = Payload::Select { retirement: after.retirement.ok_or(ManagedError::Corrupt)?,
            source: self.engine.catalog.state.current.clone().ok_or(ManagedError::Corrupt)?, target: target.foot.clone(),
            binary: self.engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.manifest_digest(), transcript, proof };
        if let Err(error) = self.engine.catalog.append(Action { payload, delta: Delta::default() }, after, PendingChange::Keep) {
            self.engine.stop(); return Err(error);
        }
        self.engine.book.clear(); self.engine.stamp = None; self.engine.world = Some(target);
        Ok(UnclaimedCoordinator { engine: self.engine })
    }
    pub fn graceful_close(mut self) -> Result<CatalogFrontier> {
        let target = self.verify()?;
        if let Err(error) = target.close() { self.engine.stop(); return Err(error); }
        self.engine.book.clear(); self.engine.catalog.close()
    }
}
