use super::{auth, bytes::hash, effect, engine::Engine, error::{ManagedError, Result},
    mutation::{self, Derived, Mutation}, pending::Pending, retirement::Retirement, rows,
    schema::Profile, state::FullState};
use std::collections::BTreeMap;

pub(super) fn verify(engine: &Engine, pending: &Pending, post: bool) -> Result<[u8; 32]> {
    pending.validate()?; engine.catalog.verify_cached()?;
    if engine.catalog.pending.as_ref().map(Pending::digest).transpose()? != Some(pending.digest()?)
        || engine.catalog.state.pending != Some(pending.digest()?) { return Err(ManagedError::Corrupt); }
    let before = FullState::decode(&pending.pre_state, pending.pre.known.store_revision)?;
    let (owned, payload, proof) = auth::decode_action(&pending.action)?;
    let cancellations = if pending.purpose == 8 {
        mutation::verify_owner(&before, &owned, &proof, nf_identity::model::ProtectedOperation::Administration)?;
        let record = Retirement::decode(&payload)?;
        if record.stamp != owned.stamp || record.sequence != pending.sequence
            || record.operation != pending.operation || record.source_foot != pending.pre
            || record.target_foot != *engine.catalog.state.target.as_ref().ok_or(ManagedError::Corrupt)?
            || record.installation != engine.catalog.state.installation || record.source != engine.catalog.state.source
            || Some(hash(&payload)) != pending.intended.retirement || pending.pre_state != pending.post_state {
            return Err(ManagedError::DerivationMismatch);
        }
        verify_retirement_files(engine, &record)?; [0; 32]
    } else {
        let mutation = Mutation::decode(pending.purpose, &payload)?;
        mutation::verify_owner(&before, &owned, &proof, mutation.role())?;
        match mutation.derive(&before, &pending.pre, &owned)? {
            Derived::Applied { state, cancellations, .. } if state.encode()? == pending.post_state => cancellations,
            _ => return Err(ManagedError::DerivationMismatch),
        }
    };
    let world = engine.world()?;
    world.verify(if post { &pending.post } else { &pending.pre }, if post { pending.intended.retirement } else { None })?;
    for table in &pending.effect.tables {
        let descriptor = rows::table(&table.name)?;
        if !post && pending.purpose == 8 && table.name == "owner_retirement" { continue; }
        let count = effect::count(&world.connection, descriptor, None)?;
        let digest = rows::digest(&world.connection, &[descriptor], &BTreeMap::new())?;
        if (count, digest) != if post { (table.post_count, table.post) } else { (table.pre_count, table.pre) } {
            return Err(ManagedError::DerivationMismatch);
        }
    }
    if !post {
        let (plan, foot) = if pending.purpose == 8 {
            super::retire::plan(engine, &Retirement::decode(&payload)?)?
        } else {
            let mutation = Mutation::decode(pending.purpose, &payload)?;
            let Derived::Applied { state, batch, .. } = mutation.derive(&before, &pending.pre, &owned)?
                else { return Err(ManagedError::DerivationMismatch); };
            let plan = world.plan(state, &mutation, &pending.action, batch.as_ref())?;
            let foot = plan.foot(&world.connection, if pending.pre.profile == 1 { Profile::Source } else { Profile::Active },
                pending.pre.lineage, &Default::default())?;
            (plan, foot)
        };
        if foot != pending.post || effect::derive(&world.connection, &pending.pre, &foot, pending.purpose,
            &plan.patches)?.encode()? != pending.effect.encode()? { return Err(ManagedError::DerivationMismatch); }
    }
    Ok(cancellations)
}
fn verify_retirement_files(engine: &Engine, record: &Retirement) -> Result<()> {
    for (kind, id, foot) in [(2, record.retained, &record.source_foot), (3, record.target, &record.target_foot)] {
        let row = engine.catalog.maps.registry.get(&kind).ok_or(ManagedError::Corrupt)?;
        let manifest = super::registry::Manifest::decode(&row.manifest)?;
        if row.artifact != id { return Err(ManagedError::SourceMismatch); }
        super::file_io::verify(&super::input::FilePin::new(row.path.clone(), row.bytes, row.digest, &manifest.provenance)?)?;
        super::readonly::verify(&row.path, foot, None)?;
    }
    engine.verify_binary(4)?; engine.verify_binary(5)?;
    if record.old_binary != engine.catalog.maps.registry.get(&4).ok_or(ManagedError::Corrupt)?.digest
        || record.new_binary != engine.catalog.maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.digest {
        return Err(ManagedError::SourceMismatch);
    }
    Ok(())
}
