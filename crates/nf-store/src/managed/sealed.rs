use super::{
    adoption, book,
    bytes::hash,
    catalog::PendingChange,
    catalog_action::Action,
    catalog_command::Command,
    catalog_payload::Payload,
    challenge::Challenge,
    delta::{Delta, RegistryPurpose},
    engine::Engine,
    error::{ManagedError, Result},
    failure::PrecommitFailure,
    owner::SourceOwner,
    prepared::{self, PreparedActiveUpgrade, Registration},
    reconcile::ReconciliationHandle,
    stamp::CatalogFrontier,
    state::QuiescePolicy,
    upgrade,
};
use crate::{
    KnownFrontiers,
    migration::{Composition, ProviderVersion},
};
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

pub(super) struct Stage {
    pub delta: Delta,
    pub target: super::foot::StorageFoot,
    pub retained: super::input::FilePin,
    pub target_pin: super::input::FilePin,
    pub retained_id: [u8; 16],
    pub target_id: [u8; 16],
}
pub struct ManagedDrainedSource {
    pub(super) engine: Engine,
    pub(super) stage: Option<Stage>,
    pub(super) retirement: Option<super::retire::RetirementDraft>,
}
fn minimum(current: KnownFrontiers, requested: KnownFrontiers) -> Result<()> {
    if current.scope != requested.scope
        || current.store_revision < requested.store_revision
        || current.event_sequence < requested.event_sequence
        || current.membership_revision < requested.membership_revision
    {
        return Err(ManagedError::StaleBackup);
    }
    Ok(())
}
pub(super) fn issue_quiesce(
    engine: &mut Engine,
    policy: QuiescePolicy,
    public: &PublicIdentity,
    cutoff: u64,
) -> Result<Challenge> {
    engine.gate(false)?;
    let world = engine.world()?;
    upgrade::drained(&world.state)?;
    if world.foot.profile != 1 {
        return Err(ManagedError::UnsupportedFormat);
    }
    let command = Command::Quiesce {
        foot: world.foot.clone(),
        policy,
        cutoff,
        delta: hash(&Delta::default().encode()?),
    };
    engine.issue_owned(11, command.encode()?, public, cutoff, None)
}
pub(super) fn quiesce(
    mut source: SourceOwner,
    policy: QuiescePolicy,
    requested: KnownFrontiers,
    proof: DeviceProof,
) -> std::result::Result<ManagedDrainedSource, PrecommitFailure<SourceOwner>> {
    let prepared = (|| {
        let engine = source.inner.engine.get_mut();
        engine.gate(false)?;
        let world = engine.world()?;
        minimum(world.foot.known, requested)?;
        upgrade::drained(&world.state)?;
        let (_, transcript, unsigned) =
            engine.consume_owned(&proof, 11, None, ProtectedOperation::Administration)?;
        let bytes = unsigned.ok_or(ManagedError::Corrupt)?;
        let Command::Quiesce {
            foot,
            policy: signed_policy,
            cutoff,
            ..
        } = Command::decode(&bytes)?
        else {
            return Err(ManagedError::OperationConflict);
        };
        if policy != signed_policy || &foot != &engine.world()?.foot {
            return Err(ManagedError::StaleSession);
        }
        let mut after = engine.catalog.state.clone();
        after.phase = 3;
        after.utc = book::now_ms()?;
        if after.utc >= cutoff {
            return Err(ManagedError::Expired);
        }
        Ok((
            Action {
                payload: Payload::Quiesce {
                    foot,
                    policy,
                    cutoff,
                    transcript,
                    proof,
                },
                delta: Delta::default(),
            },
            after,
        ))
    })();
    let (action, after) = match prepared {
        Ok(value) => value,
        Err(error) => {
            if source.inner.engine.get_mut().gate(false).is_err() {
                let engine = source.inner.engine.into_inner();
                return Err(PrecommitFailure {
                    error,
                    owner: None,
                    reconciliation: Some(ReconciliationHandle::from_engine(engine)),
                });
            }
            return Err(PrecommitFailure {
                error,
                owner: Some(source),
                reconciliation: None,
            });
        }
    };
    let mut engine = source.inner.engine.into_inner();
    if let Err(error) = engine.catalog.append(action, after, PendingChange::Keep) {
        engine.stop();
        return Err(PrecommitFailure {
            error,
            owner: None,
            reconciliation: Some(ReconciliationHandle::from_engine(engine)),
        });
    }
    engine.book.clear();
    engine.prepare_reservation = None;
    Ok(ManagedDrainedSource {
        engine,
        stage: None,
        retirement: None,
    })
}
impl ManagedDrainedSource {
    fn refuse(mut self, error: ManagedError) -> PrecommitFailure<Self> {
        if self.engine.gate(true).is_err() {
            self.engine.stop();
            PrecommitFailure {
                error,
                owner: None,
                reconciliation: Some(ReconciliationHandle::from_engine(self.engine)),
            }
        } else {
            PrecommitFailure {
                error,
                owner: Some(self),
                reconciliation: None,
            }
        }
    }
    pub(super) fn from_engine(mut engine: Engine) -> Result<Self> {
        engine.gate(true)?;
        upgrade::drained(&engine.world()?.state)?;
        Ok(Self {
            engine,
            stage: None,
            retirement: None,
        })
    }
    pub fn catalog_frontier(&self) -> CatalogFrontier {
        self.engine.catalog.frontier
    }
    pub fn materialize_active_upgrade(
        mut self,
        from: &Composition,
        to: &Composition,
        supported: &[ProviderVersion],
        target: PathBuf,
        retained: PathBuf,
    ) -> std::result::Result<(Self, PreparedActiveUpgrade), PrecommitFailure<Self>> {
        let result = (|| {
            self.engine.gate(true)?;
            if self.engine.catalog.state.target.is_some() {
                return Err(ManagedError::OperationConflict);
            }
            let mut forbidden = vec![self.engine.catalog.path.as_path()];
            forbidden.extend(
                self.engine
                    .catalog
                    .maps
                    .registry
                    .values()
                    .map(|row| row.path.as_path()),
            );
            prepared::materialize(
                self.engine.world()?,
                from,
                to,
                supported,
                target,
                retained,
                &forbidden,
                &self.engine.provenance,
            )
        })();
        match result {
            Ok(prepared) => Ok((self, prepared)),
            Err(error) => Err(self.refuse(error)),
        }
    }
    fn verify_prepared(
        &mut self,
        prepared: &PreparedActiveUpgrade,
        retained: &std::path::Path,
    ) -> Result<()> {
        self.engine.gate(true)?;
        self.engine.guard(|engine| engine.verify_binary(5))?;
        prepared.verify()?;
        if prepared.source != self.engine.world()?.foot || prepared.retained_pin.path != retained {
            return Err(ManagedError::SourceMismatch);
        }
        let old = &self
            .engine
            .catalog
            .maps
            .registry
            .get(&4)
            .ok_or(ManagedError::Corrupt)?
            .manifest;
        let new = &self
            .engine
            .catalog
            .maps
            .registry
            .get(&5)
            .ok_or(ManagedError::Corrupt)?
            .manifest;
        if super::registry::Manifest::decode(old)?.composition != prepared.from.bytes
            || super::registry::Manifest::decode(new)?.composition != prepared.to.bytes
        {
            return Err(ManagedError::UnsupportedFormat);
        }
        Ok(())
    }
    pub fn issue_registration_challenge(
        &mut self,
        prepared: &PreparedActiveUpgrade,
        retained: PathBuf,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        self.verify_prepared(prepared, &retained)?;
        if self.engine.catalog.state.target.is_some()
            || !matches!(prepared.registration, Registration::DataOnly)
        {
            return Err(ManagedError::OperationConflict);
        }
        if self.stage.is_none() {
            let retained_id = book::random()?;
            let target_id = book::random()?;
            let installation = self.engine.catalog.state.installation;
            let scope = prepared.foot.known.scope;
            let delta = Delta::additions(vec![
                adoption::artifact(
                    installation,
                    scope,
                    2,
                    retained_id,
                    &prepared.retained_pin,
                    1,
                    prepared.source.schema,
                    prepared.from.bytes.clone(),
                )?,
                adoption::artifact(
                    installation,
                    scope,
                    3,
                    target_id,
                    &prepared.target_pin,
                    4,
                    prepared.foot.schema,
                    prepared.to.bytes.clone(),
                )?,
            ])?;
            delta.apply(&self.engine.catalog.maps, RegistryPurpose::Stage, None)?;
            self.stage = Some(Stage {
                delta,
                target: prepared.foot.clone(),
                retained: prepared.retained_pin.clone(),
                target_pin: prepared.target_pin.clone(),
                retained_id,
                target_id,
            });
        }
        let stage = self.stage.as_ref().ok_or(ManagedError::Corrupt)?;
        if stage.target != prepared.foot
            || stage.retained.path != retained
            || stage.target_pin.digest != prepared.target_pin.digest
            || stage.retained.digest != prepared.retained_pin.digest
        {
            return Err(ManagedError::OperationConflict);
        }
        let maps = stage
            .delta
            .apply(&self.engine.catalog.maps, RegistryPurpose::Stage, None)?;
        let delta_bytes = stage.delta.encode()?;
        let command = Command::Register {
            registry: maps.digests()?.0,
            delta: hash(&delta_bytes),
        };
        let mut challenge = self
            .engine
            .issue_owned(12, command.encode()?, public, cutoff, None)?;
        challenge.delta = delta_bytes;
        Ok(challenge)
    }
    pub fn register_inactive_artifacts(
        mut self,
        mut prepared: PreparedActiveUpgrade,
        proof: DeviceProof,
    ) -> std::result::Result<(Self, PreparedActiveUpgrade), PrecommitFailure<Self>> {
        let verified = (|| {
            self.verify_prepared(&prepared, &prepared.retained_pin.path)?;
            let stage = self.stage.as_ref().ok_or(ManagedError::OperationConflict)?;
            if stage.target != prepared.foot
                || stage.target_pin.digest != prepared.target_pin.digest
                || stage.retained.digest != prepared.retained_pin.digest
            {
                return Err(ManagedError::TargetMismatch);
            }
            let maps =
                stage
                    .delta
                    .apply(&self.engine.catalog.maps, RegistryPurpose::Stage, None)?;
            let registry = maps.digests()?.0;
            let command = Command::Register {
                registry,
                delta: hash(&stage.delta.encode()?),
            }
            .encode()?;
            let (_, transcript, _) = self.engine.consume_owned(
                &proof,
                12,
                Some(&command),
                ProtectedOperation::Administration,
            )?;
            let mut after = self.engine.catalog.state.clone();
            after.utc = book::now_ms()?;
            after.target = Some(prepared.foot.clone());
            after.registry = registry;
            Ok((
                Action {
                    payload: Payload::Register {
                        registry,
                        transcript,
                        proof,
                    },
                    delta: self
                        .stage
                        .as_ref()
                        .ok_or(ManagedError::Corrupt)?
                        .delta
                        .clone(),
                },
                after,
            ))
        })();
        let (action, after) = match verified {
            Ok(value) => value,
            Err(error) => return Err(self.refuse(error)),
        };
        if let Err(error) = self
            .engine
            .catalog
            .append(action, after, PendingChange::Keep)
        {
            self.engine.stop();
            return Err(PrecommitFailure {
                error,
                owner: None,
                reconciliation: Some(ReconciliationHandle::from_engine(self.engine)),
            });
        }
        let stage = match self.stage.take() {
            Some(value) => value,
            None => {
                self.engine.stop();
                return Err(PrecommitFailure {
                    error: ManagedError::Corrupt,
                    owner: None,
                    reconciliation: Some(ReconciliationHandle::from_engine(self.engine)),
                });
            }
        };
        prepared.registration = Registration::Registered {
            installation: self.engine.catalog.state.installation,
            source: self.engine.catalog.state.source,
            retained: stage.retained_id,
            target: stage.target_id,
        };
        self.engine.book.clear();
        Ok((self, prepared))
    }
    /// Reconstruct only registered immutable data after a fresh sealed Claim10.
    /// This does not create, adopt, select or grant a target writer.
    pub fn recover_registered_upgrade(&mut self) -> Result<PreparedActiveUpgrade> {
        self.engine.gate(true)?;
        if self.engine.catalog.state.target.is_none() {
            return Err(ManagedError::OperationConflict);
        }
        let result = self.recover_registered();
        if result.is_err() {
            self.engine.stop();
        }
        result
    }
    fn recover_registered(&mut self) -> Result<PreparedActiveUpgrade> {
        self.engine.gate(true)?;
        upgrade::drained(&self.engine.world()?.state)?;
        let catalog = &self.engine.catalog;
        let foot = catalog
            .state
            .target
            .clone()
            .ok_or(ManagedError::OperationConflict)?;
        let source = self.engine.world()?.foot.clone();
        let retained = catalog.maps.registry.get(&2).ok_or(ManagedError::Corrupt)?;
        let target = catalog.maps.registry.get(&3).ok_or(ManagedError::Corrupt)?;
        let retained_manifest = super::registry::Manifest::decode(&retained.manifest)?;
        let target_manifest = super::registry::Manifest::decode(&target.manifest)?;
        let retained_pin = super::input::FilePin::new(
            retained.path.clone(),
            retained.bytes,
            retained.digest,
            &retained_manifest.provenance,
        )?;
        let target_pin = super::input::FilePin::new(
            target.path.clone(),
            target.bytes,
            target.digest,
            &target_manifest.provenance,
        )?;
        super::file_io::verify(&retained_pin)?;
        super::file_io::verify(&target_pin)?;
        super::readonly::verify(&retained.path, &source, None)?;
        let state = super::readonly::verify(&target.path, &foot, None)?;
        let from = super::upgrade::Descriptor::read(&retained_manifest.composition)?;
        let to = super::upgrade::Descriptor::read(&target_manifest.composition)?;
        let expected = upgrade::derive(
            &self.engine.world()?.state,
            &source,
            &from,
            &to,
            &from.providers,
        )?;
        if expected.state != state
            || expected.head != foot.head
            || Some(hash(&expected.lineage)) != foot.lineage
        {
            return Err(ManagedError::TargetMismatch);
        }
        let prepared = PreparedActiveUpgrade {
            core: crate::codec::encode(&state.core)?,
            envelope: state.encode()?,
            state,
            foot,
            source,
            target_pin,
            retained_pin,
            from,
            to,
            registration: Registration::Registered {
                installation: catalog.state.installation,
                source: catalog.state.source,
                retained: retained.artifact,
                target: target.artifact,
            },
        };
        self.verify_prepared(&prepared, &prepared.retained_pin.path)?;
        Ok(prepared)
    }
    pub fn issue_retirement_challenge(
        &mut self,
        prepared: &PreparedActiveUpgrade,
        retained: PathBuf,
        public: &PublicIdentity,
        cutoff: u64,
    ) -> Result<Challenge> {
        super::retire::issue(self, prepared, retained, public, cutoff)
    }
    pub fn retire_source(
        self,
        prepared: PreparedActiveUpgrade,
        retained: PathBuf,
        proof: DeviceProof,
    ) -> std::result::Result<super::selection::SourceFenced, super::failure::UpgradeOutcome> {
        super::retire::execute(self, prepared, retained, proof)
    }
    pub fn graceful_close(mut self) -> Result<CatalogFrontier> {
        self.engine.gate(true)?;
        let world = self
            .engine
            .world
            .take()
            .ok_or(ManagedError::ReconciliationRequired)?;
        if let Err(error) = world.close() {
            self.engine.stop();
            return Err(error);
        }
        self.engine.book.clear();
        self.engine.stamp = None;
        self.engine.catalog.close()
    }
}
