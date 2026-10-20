//! Bounded local descriptor encoding for inactive staging; no wire/authority registration.
use super::{Composition, ProviderVersion};
use crate::{schema, staging::StageError};
use nf_contract::identity::ProviderId;
use nf_kernel::World;
const PREFIX: &[u8] = b"NF-MIGRATION-COMPOSITION-1\0";
pub(crate) const MAX_BYTES: usize = 1923;

pub(crate) fn encode(value: &Composition) -> Vec<u8> {
    let mut bytes = PREFIX.to_vec();
    bytes.extend_from_slice(&value.storage_schema.to_le_bytes());
    bytes.extend_from_slice(&value.schema);
    bytes.extend_from_slice(&value.config);
    bytes.extend_from_slice(&value.semantic_content);
    bytes.extend_from_slice(&(value.providers.len() as u32).to_le_bytes());
    for provider in &value.providers {
        bytes.extend_from_slice(provider.provider.as_bytes());
        bytes.extend_from_slice(&provider.implementation);
        bytes.extend_from_slice(&provider.version.to_le_bytes());
        bytes.extend_from_slice(&provider.state_schema.to_le_bytes());
    }
    bytes
}
pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) at: usize,
}
impl Reader<'_> {
    pub(crate) fn fixed<const N: usize>(&mut self) -> Result<[u8; N], StageError> {
        let end = self.at.checked_add(N).ok_or(StageError::Corrupt)?;
        let bytes = self.bytes.get(self.at..end).ok_or(StageError::Corrupt)?;
        self.at = end;
        bytes.try_into().map_err(|_| StageError::Corrupt)
    }
    pub(crate) fn u32(&mut self) -> Result<u32, StageError> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }
}
pub(crate) fn decode(bytes: &[u8]) -> Result<Composition, StageError> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(PREFIX) {
        return Err(StageError::Corrupt);
    }
    let mut input = Reader {
        bytes,
        at: PREFIX.len(),
    };
    let storage = input.u32()?;
    let schema = input.fixed()?;
    let config = input.fixed()?;
    let content = input.fixed()?;
    let count = input.u32()?;
    if count == 0 || count > 32 {
        return Err(StageError::Corrupt);
    }
    let mut providers = Vec::new();
    for _ in 0..count {
        providers.push(ProviderVersion {
            provider: ProviderId::from_bytes(input.fixed()?),
            implementation: input.fixed()?,
            version: input.u32()?,
            state_schema: input.u32()?,
        });
    }
    let value = Composition::new(storage, schema, config, content, providers)
        .map_err(|_| StageError::Corrupt)?;
    if input.at != bytes.len() || encode(&value) != bytes {
        return Err(StageError::Corrupt);
    }
    Ok(value)
}
pub(crate) fn validate(
    from: &Composition,
    to: &Composition,
    world: &World,
) -> Result<(), StageError> {
    let actual = world.to_spec();
    if from.digest() != actual.ruleset_hash
        || from.storage_schema != schema::SCHEMA_VERSION as u32
        || from.schema != schema::schema_digest()
        || to.storage_schema != from.storage_schema
        || to.schema != from.schema
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
        return Err(StageError::UnsupportedTransition);
    }
    Ok(())
}
