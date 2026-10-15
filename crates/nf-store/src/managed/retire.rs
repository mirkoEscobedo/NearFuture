use super::{auth, book, bytes::{hash, next}, catalog::PendingChange, catalog_action::Action,
    catalog_payload::Payload, challenge::Challenge, delta::{Delta, RegistryPurpose}, effect,
    engine::Engine, error::{ManagedError, Result}, failure::UpgradeOutcome, maps::ReceiptRow,
    pending::Pending, prepared::{PreparedActiveUpgrade, Registration}, receipt::Receipt,
    reconcile::ReconciliationHandle, retirement::Retirement, rows::{self, Cell, Patch},
    schema::{self, Profile}, sealed::ManagedDrainedSource, selection::SourceFenced,
    source_owner::SourceOwner, summary::Summary, upgrade, world_plan::Plan};
use nf_identity::model::{DeviceProof, ProtectedOperation, PublicIdentity};
use std::{collections::{BTreeMap, BTreeSet}, path::PathBuf};

pub(super) struct RetirementDraft { pub record: Retirement }
fn registered(source: &mut ManagedDrainedSource, prepared: &PreparedActiveUpgrade,
    retained: &std::path::Path) -> Result<()> {
    source.engine.gate(true)?;
    let Registration::Registered { installation, source: original, retained: saved, target } = prepared.registration
        else { return Err(ManagedError::OperationConflict); };
    let state = &source.engine.catalog.state;
    if state.installation != installation || state.source != original || state.target.as_ref() != Some(&prepared.foot)
        || prepared.source != source.engine.world()?.foot || prepared.retained_pin.path != retained {
        return Err(ManagedError::SourceMismatch);
    }
    for (kind, id, pin) in [(2, saved, &prepared.retained_pin), (3, target, &prepared.target_pin)] {
        let row = source.engine.catalog.maps.registry.get(&kind).ok_or(ManagedError::Corrupt)?;
        if row.artifact != id || row.path != pin.path || row.bytes != pin.bytes || row.digest != pin.digest {
            return Err(ManagedError::TargetMismatch);
        }
    }
    source.engine.guard(|engine| {
        engine.verify_binary(4)?; engine.verify_binary(5)?; prepared.verify()?;
        upgrade::drained(&engine.world()?.state)
    })
}
pub(super) fn issue(source: &mut ManagedDrainedSource, prepared: &PreparedActiveUpgrade,
    retained: PathBuf, public: &PublicIdentity, cutoff: u64) -> Result<Challenge> {
    registered(source, prepared, &retained)?;
    let Registration::Registered { source: original, retained: saved, target, .. } = prepared.registration
        else { return Err(ManagedError::OperationConflict); };
    let sequence = next(source.engine.catalog.state.consumed)?; let operation = book::random()?;
    let record = Retirement { installation: source.engine.catalog.state.installation,
        stamp: source.engine.owned_stamp()?.clone(), sequence, operation, source: original, retained: saved, target,
        binding: upgrade::source_binding(&prepared.source)?, source_foot: prepared.source.clone(),
        target_foot: prepared.foot.clone(), composition: prepared.to.bytes.clone(),
        lineage: prepared.foot.lineage.ok_or(ManagedError::Corrupt)?,
        old_binary: source.engine.catalog.maps.registry.get(&4).ok_or(ManagedError::Corrupt)?.digest,
        new_binary: source.engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.digest };
    let challenge = source.engine.issue_owned(8, record.encode()?, public, cutoff, Some((sequence, operation)))?;
    source.retirement = Some(RetirementDraft { record }); Ok(challenge)
}
pub(super) fn plan(engine: &Engine, record: &Retirement) -> Result<(Plan, super::foot::StorageFoot)> {
    let world = engine.world()?;
    if record.source_foot != world.foot { return Err(ManagedError::SourceMismatch); }
    let bytes = record.encode()?; let mut patch = Patch::default();
    patch.put(rows::table("owner_retirement")?, None,
        Some(vec![Cell::Integer(1), Cell::Blob(bytes.clone()), Cell::Blob(hash(&bytes).to_vec())]))?;
    let plan = Plan { state: world.state.clone(), revision: world.foot.known.store_revision,
        head: world.foot.head, checkpoint: world.foot.checkpoint.clone(),
        patches: BTreeMap::from([("owner_retirement".to_string(), patch)]) };
    let foot = plan.foot(&world.connection, Profile::Retired, world.foot.lineage,
        &BTreeSet::from(["owner_retirement"]))?;
    Ok((plan, foot))
}
pub(super) fn execute(mut source: ManagedDrainedSource, prepared: PreparedActiveUpgrade,
    retained: PathBuf, proof: DeviceProof) -> std::result::Result<SourceFenced, UpgradeOutcome> {
    let admission = (|| {
        registered(&mut source, &prepared, &retained)?;
        let record = source.retirement.as_ref().ok_or(ManagedError::OperationConflict)?.record.clone();
        let payload = record.encode()?;
        let (owned, _, _) = source.engine.consume_owned(&proof, 8, Some(&payload), ProtectedOperation::Administration)?;
        if record.stamp != owned.stamp || record.sequence != owned.sequence || record.operation != owned.operation {
            return Err(ManagedError::StaleSession);
        }
        let (plan, post) = plan(&source.engine, &record)?;
        let action = auth::action(&owned, &payload, &proof)?;
        let effect = effect::derive(&source.engine.world()?.connection, &record.source_foot, &post, 8, &plan.patches)?;
        let utc = book::now_ms()?;
        if utc < owned.times.issued || utc >= owned.times.expires || utc < source.engine.catalog.state.utc {
            return Err(ManagedError::Expired);
        }
        let receipt = Receipt { sequence: owned.sequence, operation: owned.operation, purpose: 8, result: 3,
            foot: post.clone(), action: hash(&action), revision: source.engine.catalog.frontier.revision
                .checked_add(3).ok_or(ManagedError::Limit)?, cancellations: [0; 32] };
        let delta = Delta::outcome(&source.engine.catalog.maps, ReceiptRow::new(&receipt)?)?;
        let maps = delta.apply(&source.engine.catalog.maps, RegistryPurpose::None, Some(&ReceiptRow::new(&receipt)?))?;
        let mut intended = source.engine.catalog.state.clone(); intended.phase = 5; intended.owner = None;
        intended.current = Some(post.clone()); intended.retirement = Some(hash(&payload));
        intended.pending = None; intended.consumed = owned.sequence; intended.utc = utc;
        intended.receipts = maps.digests()?.1;
        let pending = Pending { sequence: owned.sequence, operation: owned.operation, purpose: 8,
            stamp: owned.stamp, before: source.engine.catalog.frontier, artifact: record.source,
            pre: record.source_foot, post, pre_state: source.engine.world()?.envelope_bytes.clone(),
            post_state: plan.state.encode()?, action, intended, effect, stage: 1 };
        pending.encode()?; Ok((plan, pending, receipt, delta))
    })();
    let (plan, pending, receipt, delta) = match admission { Ok(value) => value,
        Err(error) => {
            if !matches!(error, ManagedError::Busy | ManagedError::UncertainOutcome | ManagedError::Quarantined
                | ManagedError::ReconciliationRequired | ManagedError::Corrupt | ManagedError::SourceMismatch)
                && source.engine.gate(true).is_ok()
                && source.engine.world().is_ok_and(|world| upgrade::drained(&world.state).is_ok()) {
                return Err(UpgradeOutcome::SealedReturned { error, source });
            }
            return Err(stopped(source.engine, error));
        } };
    let mut admitted = source.engine.catalog.state.clone(); admitted.phase = 4;
    admitted.pending = match pending.digest() { Ok(value) => Some(value), Err(error) => return Err(stopped(source.engine, error)) };
    admitted.consumed = pending.sequence; admitted.utc = pending.intended.utc;
    let summary = match Summary::from_pending(&pending) { Ok(value) => value, Err(error) => return Err(stopped(source.engine, error)) };
    if let Err(error) = source.engine.catalog.append(Action { payload: Payload::Pending(summary), delta: Delta::default() },
        admitted, PendingChange::Set(pending.clone())) { return Err(stopped(source.engine, error)); }
    match attempt(&mut source.engine, &plan, &pending) {
        RetireAttempt::KnownPre(error) => {
            if let Err(abort_error) = abort_live(&mut source.engine, &pending) { return Err(stopped(source.engine, abort_error)); }
            let recovery = ReconciliationHandle::from_catalog(&source.engine.catalog, None);
            match SourceOwner::from_engine(source.engine) {
                Ok(owner) => Err(UpgradeOutcome::SourceReturned { error, source: owner }),
                Err(error) => Err(UpgradeOutcome::Reconciliation { error,
                    handle: recovery }),
            }
        },
        RetireAttempt::KnownPost(stage2) => {
            if let Err(error) = finish(&mut source.engine, &stage2, receipt, delta) { return Err(stopped(source.engine, error)); }
            let recovery = ReconciliationHandle::from_catalog(&source.engine.catalog, None);
            SourceFenced::from_engine(source.engine).map_err(|error| UpgradeOutcome::Reconciliation { error,
                handle: recovery })
        },
        RetireAttempt::Unknown(error) => Err(stopped(source.engine, error)),
    }
}
pub(super) enum RetireAttempt { KnownPre(ManagedError), KnownPost(Pending), Unknown(ManagedError) }
pub(super) fn attempt(engine: &mut Engine, plan: &Plan, pending: &Pending) -> RetireAttempt {
    match attempt_inner(engine, plan, pending) {
        Ok(result) => result, Err(error) => RetireAttempt::Unknown(error),
    }
}
fn attempt_inner(engine: &mut Engine, plan: &Plan, pending: &Pending) -> Result<RetireAttempt> {
    let world = engine.world.as_mut().ok_or(ManagedError::ReconciliationRequired)?;
    world.verify(&pending.pre, None)?;
    if !world.connection.is_autocommit() { return Err(ManagedError::Quarantined); }
    if let Err(error) = world.connection.execute_batch("BEGIN EXCLUSIVE") {
        if !world.connection.is_autocommit() { world.connection.execute_batch("ROLLBACK")?; }
        world.verify(&pending.pre, None)?; return Ok(RetireAttempt::KnownPre(error.into()));
    }
    let write = (|| {
        let ddl = schema::RETIRED_SCHEMA.split("CREATE TABLE owner_retirement").nth(1).ok_or(ManagedError::Corrupt)?;
        world.connection.execute_batch(&format!("CREATE TABLE owner_retirement{ddl}"))?;
        plan.apply(&world.connection)?; world.connection.pragma_update(None, "user_version", 5)?;
        super::world_file::inspect(&world.connection, &pending.post, pending.intended.retirement)?;
        Ok(())
    })();
    if let Err(error) = write {
        world.connection.execute_batch("ROLLBACK")?;
        world.verify(&pending.pre, None)?; return Ok(RetireAttempt::KnownPre(error));
    }
    let mut stage2 = pending.clone(); stage2.stage = 2;
    let summary = Summary::from_pending(&stage2)?;
    let mut state = engine.catalog.state.clone(); state.pending = Some(stage2.digest()?);
    if pending.stage == 1 {
    if let Err(error) = engine.catalog.append(Action { payload: Payload::Attempt(summary), delta: Delta::default() },
        state, PendingChange::Set(stage2.clone())) {
        let _rollback = world.connection.execute_batch("ROLLBACK");
        return Ok(RetireAttempt::Unknown(error));
    }
    } else if pending.stage != 2 || engine.catalog.state.pending != Some(pending.digest()?) {
        let _rollback = world.connection.execute_batch("ROLLBACK");
        return Err(ManagedError::ReconciliationRequired);
    }
    // Durable stage2 is known before the single source COMMIT attempt.
    if world.connection.execute_batch("COMMIT").is_err() { return Ok(RetireAttempt::Unknown(ManagedError::UncertainOutcome)); }
    let state = super::world_file::inspect(&world.connection, &stage2.post, stage2.intended.retirement)?;
    world.state = state; world.foot = stage2.post.clone();
    world.core_bytes = crate::codec::encode(&world.state.core)?; world.envelope_bytes = world.state.encode()?;
    Ok(RetireAttempt::KnownPost(stage2))
}
pub(super) fn finish(engine: &mut Engine, pending: &Pending, receipt: Receipt, delta: Delta) -> Result<()> {
    if pending.stage != 2 { return Err(ManagedError::OperationConflict); }
    engine.world.take().ok_or(ManagedError::ReconciliationRequired)?.close()?;
    engine.catalog.append(Action { payload: Payload::Outcome { summary: Summary::from_pending(pending)?,
        receipt, disposition: 2 }, delta }, pending.intended.clone(), PendingChange::Clear)?;
    engine.book.clear(); engine.stamp = None; engine.prepare_reservation = None; Ok(())
}
fn abort_live(engine: &mut Engine, pending: &Pending) -> Result<()> {
    if pending.stage != 1 || engine.catalog.pending.as_ref().is_none_or(|p| p.stage != 1) {
        return Err(ManagedError::ReconciliationRequired);
    }
    engine.world()?.verify(&pending.pre, None)?;
    let receipt = Receipt { sequence: pending.sequence, operation: pending.operation, purpose: 8, result: 2,
        foot: pending.pre.clone(), action: hash(&pending.action), revision: next(engine.catalog.frontier.revision)?,
        cancellations: [0; 32] };
    let delta = Delta::outcome(&engine.catalog.maps, ReceiptRow::new(&receipt)?)?;
    let maps = delta.apply(&engine.catalog.maps, RegistryPurpose::None, Some(&ReceiptRow::new(&receipt)?))?;
    let mut after = engine.catalog.state.clone(); after.phase = 2; after.pending = None;
    after.current = Some(pending.pre.clone()); after.receipts = maps.digests()?.1;
    engine.catalog.append(Action { payload: Payload::Abort { summary: Summary::from_pending(pending)?,
        receipt, disposition: 2 }, delta }, after, PendingChange::Clear)?;
    engine.reset_after_outcome()
}
fn stopped(mut engine: Engine, error: ManagedError) -> UpgradeOutcome {
    engine.stop(); UpgradeOutcome::Reconciliation { error, handle: ReconciliationHandle::from_engine(engine) }
}
