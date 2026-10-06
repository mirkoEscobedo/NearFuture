use crate::model::*;
use alloc::{collections::BTreeSet, vec::Vec};
use nf_contract::identity::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub(crate) spec: WorldSpec,
    pub(crate) outcomes: Vec<crate::Outcome>,
}
impl World {
    pub fn new(mut spec: WorldSpec) -> Result<Self, Rejection> {
        if spec.factions.len() + spec.relations.len() + spec.markets.len() > MAX_ENTITIES
            || spec.providers.len() > MAX_PROVIDERS
            || spec.registry.len() > MAX_PROVIDERS
            || spec.ownership.len() > MAX_ENTITIES + MAX_PROVIDERS
            || spec.revisions.len() > MAX_ENTITIES + MAX_PROVIDERS
        {
            return Err(Rejection::Limit);
        }
        let faction_ids: BTreeSet<_> = spec.factions.iter().map(|v| v.id).collect();
        if spec.relations.iter().any(|v| {
            !faction_ids.contains(&v.left) || !faction_ids.contains(&v.right) || v.left == v.right
        }) || spec
            .markets
            .iter()
            .any(|v| !faction_ids.contains(&v.faction))
        {
            return Err(Rejection::InvalidReference);
        }
        if spec
            .relations
            .iter()
            .any(|v| !(-10000..=10000).contains(&v.score))
        {
            return Err(Rejection::InvalidValue);
        }
        let mut entities = BTreeSet::new();
        let mut aggregates = BTreeSet::new();
        for (id, aggregate) in spec
            .factions
            .iter()
            .map(|v| (v.id, v.aggregate))
            .chain(spec.relations.iter().map(|v| (v.id, v.aggregate)))
            .chain(spec.markets.iter().map(|v| (v.id, v.aggregate)))
        {
            if !entities.insert(id) || !aggregates.insert(aggregate) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        let mut providers = BTreeSet::new();
        for provider in &spec.providers {
            if !providers.insert(provider.id) || !aggregates.insert(provider.aggregate) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        let mut registry = BTreeSet::new();
        for manifest in &spec.registry {
            if !registry.insert(manifest.id) {
                return Err(Rejection::DuplicateIdentity);
            }
            if !providers.contains(&manifest.id) {
                return Err(Rejection::InvalidReference);
            }
            if manifest.version != 1 || manifest.state_schema != 1 {
                return Err(Rejection::UnsupportedProvider);
            }
            if manifest.max_events == 0
                || manifest.max_events > 64
                || manifest.principals.len() > 64
            {
                return Err(Rejection::Limit);
            }
            let supported = match manifest.kind {
                ProviderKind::SyntheticPeace => [Capability::SeekPeace].as_slice(),
                ProviderKind::Manual => {
                    [Capability::AdjustRelation, Capability::AdjustMarket].as_slice()
                }
            };
            if manifest.capabilities.iter().any(|v| !supported.contains(v)) {
                return Err(Rejection::UnsupportedProvider);
            }
        }
        if registry != providers {
            return Err(Rejection::InvalidReference);
        }
        let mut owned = BTreeSet::new();
        for owner in &spec.ownership {
            if !owned.insert(owner.aggregate) {
                return Err(Rejection::DuplicateIdentity);
            }
            if !aggregates.contains(&owner.aggregate)
                || !providers.contains(&owner.provider)
                || owner.ruleset_hash != spec.ruleset_hash
                || owner.activation_seq > spec.event_seq
            {
                return Err(Rejection::InvalidReference);
            }
        }
        if spec.revisions.keys().copied().collect::<BTreeSet<_>>() != aggregates {
            return Err(Rejection::InvalidReference);
        }
        spec.factions.sort_by_key(|v| v.id);
        spec.relations.sort_by_key(|v| v.id);
        spec.markets.sort_by_key(|v| v.id);
        spec.providers.sort_by_key(|v| v.id);
        spec.registry.sort_by_key(|v| v.id);
        spec.ownership.sort_by_key(|v| v.aggregate);
        Ok(Self {
            spec,
            outcomes: Vec::new(),
        })
    }
    pub fn view(&self) -> WorldView<'_> {
        WorldView(self)
    }
    pub fn to_spec(&self) -> WorldSpec {
        self.spec.clone()
    }
    pub fn outcomes(&self) -> &[crate::Outcome] {
        &self.outcomes
    }
}
#[derive(Clone, Copy)]
pub struct WorldView<'a>(pub(crate) &'a World);
impl<'a> WorldView<'a> {
    pub fn relation(self, id: EntityId) -> Option<&'a Relation> {
        self.0.spec.relations.iter().find(|v| v.id == id)
    }
    pub fn market(self, id: EntityId) -> Option<&'a Market> {
        self.0.spec.markets.iter().find(|v| v.id == id)
    }
    pub fn provider(self, id: ProviderId) -> Option<&'a ProviderState> {
        self.0.spec.providers.iter().find(|v| v.id == id)
    }
    pub fn revision(self, id: AggregateId) -> Option<AggregateRevision> {
        self.0.spec.revisions.get(&id).copied()
    }
    pub fn tick(self) -> WorldTick {
        self.0.spec.tick
    }
    pub fn event_seq(self) -> EventSeq {
        self.0.spec.event_seq
    }
}
