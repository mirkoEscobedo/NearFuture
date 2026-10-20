use super::{StrategicSupportImplementation, StrategicSupportRulesetBinding, count};
use crate::error::{Result, StoreError};
use nf_kernel::{Capability, Domain, ProviderKind, WorldSpec};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct StrategicSupportManifestV1 {
    provider_ordinal: u32,
    kind: ProviderKind,
    version: u32,
    state_schema: u32,
    read_domains: Vec<Domain>,
    write_domains: Vec<Domain>,
    capabilities: Vec<Capability>,
    principal_count: u64,
    max_events: u32,
}

impl StrategicSupportManifestV1 {
    pub fn provider_ordinal(&self) -> u32 {
        self.provider_ordinal
    }
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn state_schema(&self) -> u32 {
        self.state_schema
    }
    pub fn read_domains(&self) -> &[Domain] {
        &self.read_domains
    }
    pub fn write_domains(&self) -> &[Domain] {
        &self.write_domains
    }
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
    pub fn principal_count(&self) -> u64 {
        self.principal_count
    }
    pub fn max_events(&self) -> u32 {
        self.max_events
    }
    pub fn implementation_classification(&self) -> StrategicSupportImplementation {
        StrategicSupportImplementation::RegisteredUnverified
    }
}

#[derive(Clone, Debug)]
pub struct StrategicSupportAssignmentV1 {
    aggregate_ordinal: u32,
    provider_ordinal: u32,
    generation: u64,
    activation_event: u64,
}

impl StrategicSupportAssignmentV1 {
    pub fn aggregate_ordinal(&self) -> u32 {
        self.aggregate_ordinal
    }
    pub fn provider_ordinal(&self) -> u32 {
        self.provider_ordinal
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn activation_event(&self) -> u64 {
        self.activation_event
    }
    pub fn ruleset_binding(&self) -> StrategicSupportRulesetBinding {
        StrategicSupportRulesetBinding::MatchesPersistedRuleset
    }
}

pub(super) fn from_recovered(
    mut spec: WorldSpec,
) -> Result<(
    Vec<StrategicSupportManifestV1>,
    Vec<StrategicSupportAssignmentV1>,
)> {
    spec.registry
        .sort_by_key(|manifest| *manifest.id.as_bytes());
    spec.ownership
        .sort_by_key(|record| *record.aggregate.as_bytes());
    let mut ordinals = BTreeMap::new();
    let mut manifests = Vec::with_capacity(spec.registry.len());
    for (index, manifest) in spec.registry.into_iter().enumerate() {
        let ordinal = u32::try_from(index).map_err(|_| StoreError::Limit)?;
        ordinals.insert(manifest.id, ordinal);
        manifests.push(StrategicSupportManifestV1 {
            provider_ordinal: ordinal,
            kind: manifest.kind,
            version: manifest.version,
            state_schema: manifest.state_schema,
            read_domains: manifest.read_domains.into_iter().collect(),
            write_domains: manifest.write_domains.into_iter().collect(),
            capabilities: manifest.capabilities.into_iter().collect(),
            principal_count: count(manifest.principals.len())?,
            max_events: manifest.max_events,
        });
    }
    let mut assignments = Vec::with_capacity(spec.ownership.len());
    for (index, record) in spec.ownership.into_iter().enumerate() {
        assignments.push(StrategicSupportAssignmentV1 {
            aggregate_ordinal: u32::try_from(index).map_err(|_| StoreError::Limit)?,
            provider_ordinal: *ordinals.get(&record.provider).ok_or(StoreError::Corrupt)?,
            generation: record.generation,
            activation_event: record.activation_seq.0,
        });
    }
    Ok((manifests, assignments))
}
