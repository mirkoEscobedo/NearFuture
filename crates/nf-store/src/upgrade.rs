//! Replayable NFST3 config upgrades are inactive data, never writer or activation authority.
use crate::{
    ImmutableSnapshot, KnownFrontiers, StoreError,
    migration::{Composition, ProviderVersion, staging_record, upgrade_record},
    model::State,
    schema,
    staging::{DrainedSource, StageBinding, bind, drained},
};
use nf_identity::model::MembershipState;
use nf_kernel::World;
use std::path::Path;
mod artifact;
mod record;

/// The same closed refusal vocabulary as inactive staging.
pub type UpgradeError = crate::staging::StageError;

/// Source-bound descriptor values. Construction checks an explicit support set.
pub struct ConfigUpgradeTransition {
    binding: StageBinding,
    from: Vec<u8>,
    to: Vec<u8>,
}
/// Both outcomes retain the original sealed owner; partial targets are never removed here.
pub struct UpgradeFailure {
    source: Box<DrainedSource>,
    error: UpgradeError,
}
impl UpgradeFailure {
    pub fn error(&self) -> UpgradeError {
        self.error
    }
    pub fn into_source(self) -> DrainedSource {
        *self.source
    }
}
/// Replayed immutable values. This type has no database connection or writer conversion.
pub struct PreparedConfigUpgrade {
    binding: StageBinding,
    candidate: State,
    snapshot: ImmutableSnapshot,
    membership: Option<MembershipState>,
    known: KnownFrontiers,
}
impl PreparedConfigUpgrade {
    pub fn source_binding(&self) -> &StageBinding {
        &self.binding
    }
    pub fn world(&self) -> &World {
        &self.candidate.world
    }
    pub fn snapshot(&self) -> &ImmutableSnapshot {
        &self.snapshot
    }
    pub fn membership(&self) -> Option<&MembershipState> {
        self.membership.as_ref()
    }
    pub fn known_frontiers(&self) -> KnownFrontiers {
        self.known
    }
}
/// Exact Profile1 schema bytes plus the two ordered NFST3 table definitions.
pub fn profile3_schema_digest() -> [u8; 32] {
    let mut schema = schema::SCHEMA.as_bytes().to_vec();
    schema.extend_from_slice(artifact::DDL.as_bytes());
    schema::hash(&schema)
}
impl DrainedSource {
    pub fn config_upgrade_transition(
        &self,
        from: &Composition,
        to: &Composition,
        supported: &[ProviderVersion],
    ) -> Result<ConfigUpgradeTransition, UpgradeError> {
        if bind(&self.source, self.binding.known)? != self.binding {
            return Err(UpgradeError::SourceMismatch);
        }
        upgrade_record::validate(from, to, &self.source.state.world)?;
        from.require_supported(supported)
            .map_err(|_| UpgradeError::UnsupportedTransition)?;
        to.require_supported(supported)
            .map_err(|_| UpgradeError::UnsupportedTransition)?;
        let transition = ConfigUpgradeTransition {
            binding: self.binding,
            from: staging_record::encode(from),
            to: staging_record::encode(to),
        };
        record::encode(&transition)?;
        Ok(transition)
    }
    /// Requires caller-owned, quiescent local rollback-journal storage and an absent output.
    /// No host quiescence, durable owner selection, fencing or activation is inferred.
    pub fn materialize_config_upgrade(
        self,
        transition: ConfigUpgradeTransition,
        path: impl AsRef<Path>,
    ) -> Result<(Self, PreparedConfigUpgrade), UpgradeFailure> {
        let result = (|| {
            if transition.binding != self.binding
                || bind(&self.source, self.binding.known)? != self.binding
            {
                return Err(UpgradeError::SourceMismatch);
            }
            let candidate = derive(&self.source.state, &transition)?;
            let revision = self
                .binding
                .known
                .store_revision
                .checked_add(1)
                .ok_or(StoreError::Limit)?;
            let snapshot = ImmutableSnapshot::from_state(revision, &candidate)?;
            let action = record::encode(&transition)?;
            let minimum = KnownFrontiers {
                store_revision: revision,
                ..self.binding.known
            };
            let intended = candidate.world.to_spec().ruleset_hash;
            artifact::write(&self.source, path.as_ref(), &transition, &action, &snapshot)?;
            verify_config_upgrade(path, &self.binding, intended, minimum)
        })();
        match result {
            Ok(prepared) => Ok((self, prepared)),
            Err(error) => Err(UpgradeFailure {
                source: Box::new(self),
                error,
            }),
        }
    }
}
fn derive(source: &State, transition: &ConfigUpgradeTransition) -> Result<State, UpgradeError> {
    drained(source)?;
    let from = staging_record::decode(&transition.from)?;
    let to = staging_record::decode(&transition.to)?;
    upgrade_record::validate(&from, &to, &source.world)?;
    let mut candidate = source.clone();
    candidate.world = source
        .world
        .with_ruleset_hash(to.digest())
        .map_err(|_| UpgradeError::UnsupportedTransition)?;
    candidate.validate()?;
    Ok(candidate)
}
/// Independently replays the preserved base and its single config transition.
/// Supply the exact source binding, intended target digest and protected target minima.
/// This read-only observation grants no compatibility, owner selection or writing authority.
pub fn verify_config_upgrade(
    path: impl AsRef<Path>,
    expected: &StageBinding,
    intended_target: [u8; 32],
    minimum: KnownFrontiers,
) -> Result<PreparedConfigUpgrade, UpgradeError> {
    artifact::verify(path.as_ref(), expected, intended_target, minimum)
}
