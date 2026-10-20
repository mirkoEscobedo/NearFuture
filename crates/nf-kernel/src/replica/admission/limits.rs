use crate::{Intent, MAX_FRONTIER, Rejection, World};

pub(super) fn frontier(world: &World, intents: &[Intent]) -> Result<(), Rejection> {
    if intents.len() > MAX_FRONTIER {
        return Err(Rejection::Limit);
    }
    let mut entries = snapshot_entries(world)?;
    add(&mut entries, intents.len())?;
    for intent in intents {
        add(&mut entries, intent.expected.len())?;
    }
    if entries > 16384 {
        return Err(Rejection::Limit);
    }
    Ok(())
}

fn snapshot_entries(world: &World) -> Result<usize, Rejection> {
    let spec = &world.spec;
    let mut entries = 0;
    for count in [
        spec.factions.len(),
        spec.relations.len(),
        spec.markets.len(),
        spec.providers.len(),
        spec.registry.len(),
        spec.ownership.len(),
        spec.revisions.len(),
        world.outcomes.len(),
    ] {
        add(&mut entries, count)?;
    }
    for manifest in &spec.registry {
        for count in [
            manifest.read_domains.len(),
            manifest.write_domains.len(),
            manifest.capabilities.len(),
            manifest.principals.len(),
        ] {
            add(&mut entries, count)?;
        }
    }
    Ok(entries)
}

fn add(total: &mut usize, count: usize) -> Result<(), Rejection> {
    *total = total.checked_add(count).ok_or(Rejection::Limit)?;
    Ok(())
}
