use crate::{BoundaryError, Definition, Snapshot, Surface, classify_definition};
use alloc::collections::BTreeSet;

const CONCERN_TAGS: [&str; 5] = [
    "diplomacy",
    "canMakePeace",
    "trait_pacifist",
    "trait_weak-willed",
    "!trait_stalwart",
];
const ACTION_TAGS: [&str; 3] = ["diplomacy", "canMakePeace", "friendly"];

#[derive(Default)]
pub(crate) struct Budget {
    entries: usize,
    text_bytes: usize,
}
impl Budget {
    pub(crate) fn entries(&mut self, count: usize, maximum: usize) -> Result<(), BoundaryError> {
        self.entries = self
            .entries
            .checked_add(count)
            .ok_or(BoundaryError::Limit)?;
        if count > maximum || self.entries > 8192 {
            return Err(BoundaryError::Limit);
        }
        Ok(())
    }
    pub(crate) fn text(&mut self, value: &str) -> Result<(), BoundaryError> {
        self.text_bytes = self
            .text_bytes
            .checked_add(value.len())
            .ok_or(BoundaryError::Limit)?;
        if value.is_empty() || value.len() > 256 || self.text_bytes > 1_048_576 {
            return Err(BoundaryError::Limit);
        }
        if value.chars().any(char::is_control)
            || !nf_contract::canonical::text::is_canonical_text(value)
        {
            return Err(BoundaryError::InvalidFact("text"));
        }
        Ok(())
    }
    fn definition(&mut self, definition: &Definition) -> Result<Surface, BoundaryError> {
        self.text(&definition.id)?;
        self.text(&definition.class_path)?;
        classify_definition(definition)
    }
    fn strings(
        &mut self,
        values: &[alloc::string::String],
        maximum: usize,
        name: &'static str,
    ) -> Result<(), BoundaryError> {
        self.entries(values.len(), maximum)?;
        let mut unique = BTreeSet::new();
        for value in values {
            self.text(value)?;
            if !unique.insert(value) {
                return Err(BoundaryError::Duplicate(name));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate(snapshot: &Snapshot) -> Result<(), BoundaryError> {
    let mut budget = Budget::default();
    budget.text(&snapshot.provenance.source_commit)?;
    if snapshot.provenance.source_commit != crate::SOURCE_COMMIT {
        return Err(BoundaryError::InvalidFact("source pin"));
    }
    let identity = &snapshot.provenance;
    if [
        identity.source_digest,
        identity.runtime_digest,
        identity.ruleset_digest,
        identity.merged_config_digest,
    ]
    .contains(&[0; 32])
        || identity.runtime_session.0 == 0
    {
        return Err(BoundaryError::MissingFact("provenance identity"));
    }
    if budget.definition(&snapshot.concern_config.definition)? != Surface::WarWearinessInput {
        return Err(BoundaryError::InvalidFact("concern definition"));
    }
    budget.strings(&snapshot.concern_config.tags, 64, "concern tag")?;
    let tags: BTreeSet<_> = snapshot
        .concern_config
        .tags
        .iter()
        .map(|tag| tag.as_str())
        .collect();
    if tags != CONCERN_TAGS.into_iter().collect() {
        return Err(BoundaryError::InvalidFact("concern tags"));
    }
    budget.text(&snapshot.subject_faction)?;
    budget.entries(snapshot.factions.len(), 256)?;
    let mut factions = BTreeSet::new();
    for faction in &snapshot.factions {
        budget.text(&faction.id)?;
        if !factions.insert(faction.id.as_str()) {
            return Err(BoundaryError::Duplicate("faction"));
        }
        budget.strings(&faction.traits, 64, "trait")?;
        if faction
            .traits
            .iter()
            .any(|value| !matches!(value.as_str(), "pacifist" | "weak-willed" | "stalwart"))
        {
            return Err(BoundaryError::InvalidFact("faction trait"));
        }
        budget.entries(faction.priority_multipliers.len(), 64)?;
        let mut priority_ids = BTreeSet::new();
        for (id, _) in &faction.priority_multipliers {
            budget.text(id)?;
            if id != "warWeariness" {
                return Err(BoundaryError::InvalidFact("faction priority definition"));
            }
            if !priority_ids.insert(id) {
                return Err(BoundaryError::Duplicate("faction priority"));
            }
        }
    }
    if !factions.contains(snapshot.subject_faction.as_str()) {
        return Err(BoundaryError::MissingFact("subject faction"));
    }
    let reference = |id: &str| {
        if factions.contains(id) {
            Ok(())
        } else {
            Err(BoundaryError::MissingFact("faction reference"))
        }
    };
    budget.strings(&snapshot.weariness.enemies, 256, "enemy")?;
    for enemy in &snapshot.weariness.enemies {
        reference(enemy)?;
        if enemy == &snapshot.subject_faction {
            return Err(BoundaryError::InvalidFact("self enemy"));
        }
    }
    budget.entries(snapshot.relations.len(), 4096)?;
    let mut relations = BTreeSet::new();
    for relation in &snapshot.relations {
        budget.text(&relation.from)?;
        budget.text(&relation.to)?;
        reference(&relation.from)?;
        reference(&relation.to)?;
        if relation.from == relation.to {
            return Err(BoundaryError::InvalidFact("self relation"));
        }
        if !relations.insert((&relation.from, &relation.to)) {
            return Err(BoundaryError::Duplicate("relation"));
        }
    }
    budget.entries(snapshot.strengths.len(), 256)?;
    let mut strength_factions = BTreeSet::new();
    let mut market_ids = BTreeSet::new();
    for strength in &snapshot.strengths {
        budget.text(&strength.faction)?;
        reference(&strength.faction)?;
        if !strength_factions.insert(&strength.faction) {
            return Err(BoundaryError::Duplicate("strength faction"));
        }
        budget.entries(strength.markets.len(), 256)?;
        for market in &strength.markets {
            budget.text(&market.id)?;
            if !market_ids.insert(market.id.as_str()) {
                return Err(BoundaryError::Duplicate("market"));
            }
        }
    }
    budget.entries(snapshot.action_configs.len(), 64)?;
    let mut action_ids = BTreeSet::new();
    for config in &snapshot.action_configs {
        if budget.definition(&config.definition)? != Surface::MakePeaceUnavailable {
            return Err(BoundaryError::InvalidFact("action definition"));
        }
        if !action_ids.insert(config.definition.id.as_str()) {
            return Err(BoundaryError::Duplicate("action definition"));
        }
        budget.strings(&config.tags, 64, "action tag")?;
        if config
            .tags
            .iter()
            .map(|tag| tag.as_str())
            .collect::<BTreeSet<_>>()
            != ACTION_TAGS.into_iter().collect()
        {
            return Err(BoundaryError::InvalidFact("action tags"));
        }
        if let Some(id) = &config.repetition_id {
            budget.text(id)?;
        }
    }
    budget.entries(snapshot.concerns.len(), 256)?;
    let mut instances = BTreeSet::new();
    for concern in &snapshot.concerns {
        if budget.definition(&concern.definition)? != Surface::WarWearinessInput {
            return Err(BoundaryError::InvalidFact("concern definition"));
        }
        if !instances.insert(concern.instance) {
            return Err(BoundaryError::Duplicate("concern instance"));
        }
        if let Some(faction) = &concern.target_faction {
            budget.text(faction)?;
            reference(faction)?;
        }
        if let Some(market) = &concern.market_id {
            budget.text(market)?;
            if !market_ids.contains(market.as_str()) {
                return Err(BoundaryError::MissingFact("market reference"));
            }
        }
        budget.entries(concern.priority.len(), 64)?;
        let mut modifiers = BTreeSet::new();
        for modifier in &concern.priority {
            budget.text(&modifier.id)?;
            if !matches!(
                modifier.id.as_str(),
                "value"
                    | "alignment_diplomatic"
                    | "trait_pacifist"
                    | "trait_weak-willed"
                    | "trait_stalwart"
                    | "faction"
            ) {
                return Err(BoundaryError::InvalidFact("priority modifier"));
            }
            if !modifiers.insert((&modifier.id, modifier.kind as u8)) {
                return Err(BoundaryError::Duplicate("priority modifier"));
            }
        }
        if let Some(action) = &concern.action {
            if budget.definition(&action.definition)? != Surface::MakePeaceUnavailable {
                return Err(BoundaryError::InvalidFact("action definition"));
            }
            if !action_ids.contains(action.definition.id.as_str()) {
                return Err(BoundaryError::MissingFact("action config"));
            }
        }
    }
    budget.entries(snapshot.timers.intervals.len(), 64)?;
    let mut intervals = BTreeSet::new();
    for interval in &snapshot.timers.intervals {
        budget.text(&interval.id)?;
        if !matches!(
            interval.id.as_str(),
            "strategic-short" | "strategic-meeting" | "diplomacy-brain"
        ) {
            return Err(BoundaryError::InvalidFact("interval kind"));
        }
        if !intervals.insert(&interval.id) {
            return Err(BoundaryError::Duplicate("interval"));
        }
    }
    budget.entries(snapshot.timers.draws.len(), 4096)?;
    for (ordinal, draw) in snapshot.timers.draws.iter().enumerate() {
        budget.text(&draw.purpose)?;
        if !matches!(
            draw.purpose.as_str(),
            "concern-action-selection" | "diplomacy-interval" | "peace-choice"
        ) || draw.ordinal as usize != ordinal
        {
            return Err(BoundaryError::InvalidFact("random draw"));
        }
    }
    Ok(())
}
