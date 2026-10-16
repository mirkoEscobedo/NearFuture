use super::{
    active_plan, active_read,
    bytes::hash,
    error::{ManagedError, Result},
    file_io,
    foot::StorageFoot,
    input::FilePin,
    schema::{self, Profile},
    state::FullState,
    upgrade::{self, Descriptor},
    world_file::WorldFile,
};
use crate::{
    KnownFrontiers,
    migration::{Composition, ProviderVersion},
};
use nf_kernel::World;
use rusqlite::TransactionBehavior;
use std::path::{Path, PathBuf};

pub(super) enum Registration {
    DataOnly,
    Registered {
        installation: [u8; 16],
        source: [u8; 16],
        retained: [u8; 16],
        target: [u8; 16],
    },
}
/// Closed, verified data only. No field or getter constructs an owner.
pub struct PreparedActiveUpgrade {
    pub(super) state: FullState,
    pub(super) foot: StorageFoot,
    pub(super) core: Vec<u8>,
    pub(super) envelope: Vec<u8>,
    pub(super) source: StorageFoot,
    pub(super) target_pin: FilePin,
    pub(super) retained_pin: FilePin,
    pub(super) from: Descriptor,
    pub(super) to: Descriptor,
    pub(super) registration: Registration,
}
impl PreparedActiveUpgrade {
    pub fn world(&self) -> &World {
        &self.state.core.world
    }
    pub fn known_frontiers(&self) -> KnownFrontiers {
        self.foot.known
    }
    pub fn core_bytes(&self) -> &[u8] {
        &self.core
    }
    pub fn envelope_bytes(&self) -> &[u8] {
        &self.envelope
    }
    pub fn storage_foot(&self) -> &StorageFoot {
        &self.foot
    }
    pub(super) fn verify(&self) -> Result<()> {
        file_io::verify(&self.target_pin)?;
        file_io::verify(&self.retained_pin)?;
        let target = super::readonly::verify(&self.target_pin.path, &self.foot, None)?;
        if target != self.state {
            return Err(ManagedError::TargetMismatch);
        }
        super::readonly::verify(&self.retained_pin.path, &self.source, None)?;
        file_io::verify(&self.target_pin)?;
        file_io::verify(&self.retained_pin)
    }
}
pub(super) fn materialize(
    source: &WorldFile,
    from: &Composition,
    to: &Composition,
    supported: &[ProviderVersion],
    target: PathBuf,
    retained: PathBuf,
    forbidden: &[&Path],
    provenance: &str,
) -> Result<PreparedActiveUpgrade> {
    source.verify(&source.foot, None)?;
    if source.foot.profile != 1 || !source.connection.is_autocommit() {
        return Err(ManagedError::NotDrained);
    }
    let from = Descriptor::of(from)?;
    let to = Descriptor::of(to)?;
    let initial = upgrade::derive(&source.state, &source.foot, &from, &to, supported)?;
    schema::local_absolute(&target)?;
    schema::local_absolute(&retained)?;
    if target == retained
        || target == source.path
        || retained == source.path
        || forbidden
            .iter()
            .any(|path| *path == target.as_path() || *path == retained.as_path())
    {
        return Err(ManagedError::TargetMismatch);
    }
    for path in [&target, &retained] {
        schema::absent(path)?;
        schema::companions_absent(path, false)?;
    }
    let pages: u32 = source
        .connection
        .pragma_query_value(None, "page_count", |r| r.get(0))?;
    let room = source
        .state
        .quota
        .checked_sub(8192)
        .ok_or(ManagedError::Limit)?;
    if room == 0 || pages == 0 || pages > room || pages > 57_344 {
        return Err(ManagedError::Limit);
    }
    let original = file_io::observe(&source.path, provenance)?;
    let mut target_connection = file_io::backup(
        &source.connection,
        &target,
        Profile::Active,
        source.state.quota,
        pages,
        pages.div_ceil(100),
    )?;
    {
        let tx = target_connection.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        active_plan::initialize(&tx, &initial)?;
        tx.commit().map_err(|_| ManagedError::UncertainOutcome)?;
    }
    schema::physical(&target_connection, Profile::Active)?;
    super::world_file::quota_check(&target_connection, source.state.quota)?;
    let minimum = KnownFrontiers {
        store_revision: initial.revision,
        ..source.foot.known
    };
    let checked = active_read::inspect(&target_connection, minimum, None)?;
    if checked.state != initial.state
        || checked.foot.head != initial.head
        || checked.foot.lineage != Some(hash(&initial.lineage))
    {
        return Err(ManagedError::DerivationMismatch);
    }
    let foot = checked.foot;
    schema::close_sync(target_connection, &target)?;
    let retained_pin =
        file_io::retained_copy(&source.connection, &source.path, &retained, provenance)?;
    if (retained_pin.bytes, retained_pin.digest) != (original.bytes, original.digest) {
        return Err(ManagedError::SourceMismatch);
    }
    let target_pin = file_io::observe(&target, provenance)?;
    let value = PreparedActiveUpgrade {
        core: crate::codec::encode(&initial.state.core)?,
        envelope: initial.state.encode()?,
        state: initial.state,
        foot,
        source: source.foot.clone(),
        target_pin,
        retained_pin,
        from,
        to,
        registration: Registration::DataOnly,
    };
    value.verify()?;
    source.verify(&source.foot, None)?;
    file_io::verify(&original)?;
    Ok(value)
}
