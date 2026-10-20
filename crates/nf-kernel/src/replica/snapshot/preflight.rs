use crate::{Rejection, World};
use nf_contract::canonical::replica_budget::{ScopedError, ScopedResult};

struct Layout {
    bytes: usize,
    entries: usize,
}
impl Layout {
    fn rows(&mut self, count: usize, width: usize) -> Result<(), Rejection> {
        u32::try_from(count).map_err(|_| Rejection::Limit)?;
        self.entries = self.entries.checked_add(count).ok_or(Rejection::Limit)?;
        self.bytes = self.bytes.checked_add(4).ok_or(Rejection::Limit)?;
        self.bytes = self
            .bytes
            .checked_add(count.checked_mul(width).ok_or(Rejection::Limit)?)
            .ok_or(Rejection::Limit)?;
        if self.entries > 16384 || self.bytes > super::writer::MAX_BYTES {
            return Err(Rejection::Limit);
        }
        Ok(())
    }
}

pub(super) fn snapshot(world: &World) -> ScopedResult<(), Rejection> {
    checked(world).map_err(ScopedError::Semantic)
}

fn checked(world: &World) -> Result<(), Rejection> {
    let spec = &world.spec;
    // Actual canonical header17 plus the fixed World fields112.
    let mut layout = Layout {
        bytes: 129,
        entries: 0,
    };
    layout.rows(spec.factions.len(), 32)?;
    layout.rows(spec.relations.len(), 72)?;
    layout.rows(spec.markets.len(), 56)?;
    layout.rows(spec.providers.len(), 48)?;
    layout.rows(spec.registry.len(), 64)?;
    for manifest in &spec.registry {
        layout.rows(manifest.read_domains.len(), 4)?;
        layout.rows(manifest.write_domains.len(), 4)?;
        layout.rows(manifest.capabilities.len(), 4)?;
        layout.rows(manifest.principals.len(), 16)?;
    }
    layout.rows(spec.ownership.len(), 80)?;
    layout.rows(spec.revisions.len(), 24)?;
    layout.rows(world.outcomes.len(), 36)?;
    // Snapshot contains no length-prefixed child byte payload; the nested262144-byte
    // cap remains on the unchanged default frontier/batch writers, outside this seam.
    Ok(())
}
