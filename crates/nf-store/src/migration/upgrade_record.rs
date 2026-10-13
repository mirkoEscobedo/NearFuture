//! Exact Profile1-to-Profile3 config-only relation; no provider transformation or authority.
use super::Composition;
use crate::{schema, upgrade::UpgradeError};
use nf_kernel::World;

pub(crate) fn validate(
    from: &Composition,
    to: &Composition,
    world: &World,
) -> Result<(), UpgradeError> {
    let actual = world.to_spec();
    if from.digest() != actual.ruleset_hash
        || from.storage_schema != schema::SCHEMA_VERSION as u32
        || from.schema != schema::schema_digest()
        || to.storage_schema != 3
        || to.schema != crate::upgrade::profile3_schema_digest()
        || to.semantic_content != from.semantic_content
        || to.providers != from.providers
        || to.config == from.config
        || from.providers.len() != actual.registry.len()
        || from.providers.iter().zip(&actual.registry).any(|(p, r)| {
            p.provider != r.id
                || p.implementation != r.implementation_hash
                || p.version != 1
                || p.state_schema != 1
                || p.version != r.version
                || p.state_schema != r.state_schema
        })
    {
        return Err(UpgradeError::UnsupportedTransition);
    }
    Ok(())
}
