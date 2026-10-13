//! Isolated config-only staging data. This module grants no activation or writer authority.
use crate::{
    ImmutableSnapshot, KnownFrontiers, Store, StoreError,
    migration::{Composition, ProviderVersion, staging_record},
    model::{RequestStatus, State},
};
use nf_identity::{codec::encode_state, model::MembershipState};
use nf_kernel::World;
use std::path::Path;
mod artifact;
mod record;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageError {
    NotDrained,
    Quarantined,
    SourceMismatch,
    UnsupportedTransition,
    DerivationMismatch,
    Corrupt,
    UnsupportedFormat,
    Storage(StoreError),
}
impl From<StoreError> for StageError {
    fn from(error: StoreError) -> Self {
        if error == StoreError::Quarantined {
            Self::Quarantined
        } else {
            Self::Storage(error)
        }
    }
}
impl From<rusqlite::Error> for StageError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::from(error).into()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageBinding {
    known: KnownFrontiers,
    head: [u8; 32],
    snapshot: [u8; 32],
    membership: Option<[u8; 32]>,
}
impl StageBinding {
    pub fn known_frontiers(&self) -> KnownFrontiers {
        self.known
    }
}
pub struct DrainedSource {
    source: Store,
    binding: StageBinding,
}
pub struct SealRefusal {
    source: Box<Store>,
    error: StageError,
}
impl SealRefusal {
    pub fn error(&self) -> StageError {
        self.error
    }
    pub fn into_source(self) -> Store {
        *self.source
    }
}
pub struct StageFailure {
    source: Box<DrainedSource>,
    error: StageError,
}
impl StageFailure {
    pub fn error(&self) -> StageError {
        self.error
    }
    pub fn into_source(self) -> DrainedSource {
        *self.source
    }
}
/// Immutable, source-bound config delta; construction validates explicit support.
pub struct ConfigOnlyTransition {
    binding: StageBinding,
    from: Vec<u8>,
    to: Vec<u8>,
}
/// Validated inactive data. It cannot be opened here as a writable Store.
pub struct StageRecord {
    binding: StageBinding,
    candidate: State,
    snapshot: ImmutableSnapshot,
    membership: Option<MembershipState>,
}
impl StageRecord {
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
}
fn drained(state: &State) -> Result<(), StageError> {
    state.validate()?;
    if state.pending.is_some()
        || !state.reservations.is_empty()
        || state
            .requests
            .values()
            .any(|r| matches!(r.status, RequestStatus::Pending { .. }))
    {
        return Err(StageError::NotDrained);
    }
    Ok(())
}
fn membership_digest(value: Option<&MembershipState>) -> Result<Option<[u8; 32]>, StageError> {
    value
        .map(|state| {
            encode_state(state)
                .map(|bytes| crate::schema::hash(&bytes))
                .map_err(|_| StageError::Corrupt)
        })
        .transpose()
}
pub(crate) fn bind(source: &Store, minimum: KnownFrontiers) -> Result<StageBinding, StageError> {
    source.ensure_writable()?;
    drained(&source.state)?;
    let head = source.verified_source_head(minimum)?;
    let known = source.known_frontiers()?;
    let membership = crate::identity::load(&source.connection, known.scope)?;
    Ok(StageBinding {
        known,
        head,
        snapshot: source.snapshot()?.digest(),
        membership: membership_digest(membership.as_ref())?,
    })
}
pub(crate) fn seal(source: Store, minimum: KnownFrontiers) -> Result<DrainedSource, SealRefusal> {
    match bind(&source, minimum) {
        Ok(binding) => Ok(DrainedSource { source, binding }),
        Err(error) => Err(SealRefusal {
            source: Box::new(source),
            error,
        }),
    }
}
impl DrainedSource {
    pub fn binding(&self) -> StageBinding {
        self.binding
    }
    /// Returns only the original owner. Host admission quiescence remains a caller precondition.
    pub fn into_store(self) -> Store {
        self.source
    }
    pub fn config_only_transition(
        &self,
        from: &Composition,
        to: &Composition,
        supported: &[ProviderVersion],
    ) -> Result<ConfigOnlyTransition, StageError> {
        if bind(&self.source, self.binding.known)? != self.binding {
            return Err(StageError::SourceMismatch);
        }
        staging_record::validate(from, to, &self.source.state.world)?;
        from.require_supported(supported)
            .map_err(|_| StageError::UnsupportedTransition)?;
        to.require_supported(supported)
            .map_err(|_| StageError::UnsupportedTransition)?;
        Ok(ConfigOnlyTransition {
            binding: self.binding,
            from: staging_record::encode(from),
            to: staging_record::encode(to),
        })
    }
    /// Builds and verifies a separate artifact; both outcomes retain this sealed source.
    pub fn stage_config_only(
        self,
        transition: ConfigOnlyTransition,
        path: impl AsRef<Path>,
    ) -> Result<(Self, StageRecord), StageFailure> {
        let result = (|| {
            if transition.binding != self.binding
                || bind(&self.source, self.binding.known)? != self.binding
            {
                return Err(StageError::SourceMismatch);
            }
            let candidate = derive(&self.source.state, &transition)?;
            let snapshot =
                ImmutableSnapshot::from_state(self.binding.known.store_revision, &candidate)?;
            let metadata = record::encode(&transition)?;
            artifact::write(&self.source, path.as_ref(), &metadata, snapshot.bytes())?;
            verify_config_stage(path, &self.binding)
        })();
        match result {
            Ok(record) => Ok((self, record)),
            Err(error) => Err(StageFailure {
                source: Box::new(self),
                error,
            }),
        }
    }
}
fn derive(source: &State, transition: &ConfigOnlyTransition) -> Result<State, StageError> {
    drained(source)?;
    let from = staging_record::decode(&transition.from)?;
    let to = staging_record::decode(&transition.to)?;
    staging_record::validate(&from, &to, &source.world)?;
    let mut candidate = source.clone();
    candidate.world = source
        .world
        .with_ruleset_hash(to.digest())
        .map_err(|_| StageError::UnsupportedTransition)?;
    candidate.validate()?;
    Ok(candidate)
}
/// Verifies the recorded config-only relation to an exact source, not an external target selection.
/// The caller must supply an owned, quiescent local rollback-journal artifact.
/// Concurrent writers, WAL and hostile path admission are outside this contract.
///
/// Callers must compare the returned World ruleset with their intended target. No artifact,
/// support descriptor or digest grants activation, compatibility or writing authority.
pub fn verify_config_stage(
    path: impl AsRef<Path>,
    expected: &StageBinding,
) -> Result<StageRecord, StageError> {
    artifact::verify(path.as_ref(), expected)
}
