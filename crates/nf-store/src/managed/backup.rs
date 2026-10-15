use super::{catalog_command::Command, challenge::Challenge, engine::Engine,
    error::{ManagedError, Result}, file_io, input::FilePin, schema, world_file::{self, WorldFile}};
use crate::KnownFrontiers;
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::path::PathBuf;

fn validate(engine: &mut Engine, path: &std::path::Path, minimum: KnownFrontiers) -> Result<Command> {
    engine.gate(false)?; schema::local_absolute(path)?;
    if path == engine.catalog.path || engine.catalog.maps.registry.values().any(|row| row.path == path) {
        return Err(ManagedError::TargetMismatch);
    }
    schema::absent(path)?; schema::companions_absent(path, false)?;
    let world = engine.world()?;
    let pages: u32 = world.connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    let command = Command::Backup { path: path.to_path_buf(), minimum, foot: world.foot.clone(),
        max_bytes: u64::from(world.foot.quota) * 4096, pages, steps: pages.div_ceil(100) };
    command.encode()?; Ok(command)
}
pub(super) fn issue(engine: &mut Engine, path: PathBuf, minimum: KnownFrontiers,
    public: &PublicIdentity, cutoff: u64) -> Result<Challenge> {
    let command = validate(engine, &path, minimum)?;
    engine.issue_owned(15, command.encode()?, public, cutoff, None)
}
pub(super) fn execute(engine: &mut Engine, proof: DeviceProof) -> Result<FilePin> {
    let (_, _, unsigned) = engine.consume_owned(&proof, 15, None, ProtectedOperation::Administration)?;
    let bytes = unsigned.ok_or(ManagedError::Corrupt)?;
    let Command::Backup { path, minimum, foot, max_bytes: _, pages, steps } = Command::decode(&bytes)?
        else { return Err(ManagedError::OperationConflict); };
    if validate(engine, &path, minimum)?.encode()? != bytes { return Err(ManagedError::StaleSession); }
    let world = engine.world()?;
    let target = file_io::backup(&world.connection, &path, world_file::profile(foot.profile)?, foot.quota, pages, steps)?;
    world_file::inspect(&target, &foot, None)?; schema::close_sync(target, &path)?;
    let reopened = WorldFile::open(path.clone(), &foot, false, None)?; reopened.close()?;
    engine.gate(false)?; file_io::observe(&path, &engine.provenance)
}
