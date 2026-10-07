use crate::{PeaceFacts, Unavailable, WarFacts, WarOperation};
use alloc::vec::Vec;
use nf_contract::identity::{BranchId, CampaignId, EntityId, ProviderId};
use nf_nex_boundary::{Modifier, ModifierKind, Observation, Provenance};
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowMetadata {
    pub provenance: Provenance,
    pub subject_faction: alloc::string::String,
    pub concern_instance: Option<EntityId>,
    pub provider: ProviderId,
    pub campaign: CampaignId,
    pub branch: BranchId,
    pub reference_digest: [u8; 32],
    pub corpus_digest: [u8; 32],
    pub implementation_digest: [u8; 32],
    pub capture_policy_digest: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShadowInput {
    War {
        operation: WarOperation,
        facts: WarFacts,
    },
    SelectedPeace(PeaceFacts),
}
/// Private diagnostic commitment only: no NF-CANON/protobuf registry or authority grant.
pub fn input_digest(
    metadata: &ShadowMetadata,
    input: &ShadowInput,
) -> Result<[u8; 32], Unavailable> {
    Ok(Sha256::digest(encode_input(metadata, input)?).into())
}
pub fn encode_input(m: &ShadowMetadata, input: &ShadowInput) -> Result<Vec<u8>, Unavailable> {
    crate::validation::text(&m.subject_faction)?;
    if m.subject_faction.len() > 128 {
        return Err(Unavailable::Limit);
    }
    let p = &m.provenance;
    if p.source_commit != nf_nex_boundary::SOURCE_COMMIT
        || p.runtime_session.0 == 0
        || [
            p.source_digest,
            p.runtime_digest,
            p.ruleset_digest,
            p.merged_config_digest,
            m.reference_digest,
            m.corpus_digest,
            m.implementation_digest,
            m.capture_policy_digest,
        ]
        .contains(&[0; 32])
    {
        return Err(Unavailable::MissingFact);
    }
    let mut w = Writer {
        bytes: Vec::new(),
        entries: 0,
    };
    w.put(b"NF-NEX-SHADOW-1\0")?;
    w.put(&1_u16.to_le_bytes())?;
    w.text(&p.source_commit)?;
    for digest in [
        p.source_digest,
        p.runtime_digest,
        p.ruleset_digest,
        p.merged_config_digest,
        m.reference_digest,
        m.corpus_digest,
        m.implementation_digest,
        m.capture_policy_digest,
    ] {
        w.put(&digest)?;
    }
    for id in [
        p.universe.as_bytes(),
        p.history.as_bytes(),
        m.provider.as_bytes(),
        m.campaign.as_bytes(),
        m.branch.as_bytes(),
    ] {
        w.put(id)?;
    }
    w.text(&m.subject_faction)?;
    w.bool(m.concern_instance.is_some())?;
    if let Some(id) = m.concern_instance {
        w.put(id.as_bytes())?;
    }
    w.put(&p.runtime_session.0.to_le_bytes())?;
    w.put(&p.frontier.0.to_le_bytes())?;
    w.u8(match p.observation {
        Observation::Synthetic => 1,
        Observation::CapturedUnverified => 2,
    })?;
    w.u8(1)?;
    match input {
        ShadowInput::War { operation, facts } => {
            crate::validation::war(facts)?;
            w.u8(1)?;
            w.u8(match operation {
                WarOperation::Generate => 1,
                WarOperation::Update => 2,
                WarOperation::IsValid => 3,
            })?;
            war(&mut w, facts)?;
        }
        ShadowInput::SelectedPeace(facts) => {
            if facts.faction != m.subject_faction {
                return Err(Unavailable::Stale);
            }
            crate::peace::validate(facts)?;
            w.u8(2)?;
            peace(&mut w, facts)?;
        }
    }
    Ok(w.bytes)
}
pub(crate) struct Writer {
    bytes: Vec<u8>,
    entries: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Unavailable> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > 65536)
        {
            return Err(Unavailable::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn u8(&mut self, n: u8) -> Result<(), Unavailable> {
        self.put(&[n])
    }
    fn bool(&mut self, b: bool) -> Result<(), Unavailable> {
        self.u8(u8::from(b))
    }
    fn u32(&mut self, n: u32) -> Result<(), Unavailable> {
        self.put(&n.to_le_bytes())
    }
    fn f(&mut self, n: nf_nex_boundary::FloatBits) -> Result<(), Unavailable> {
        self.u32(n.bits())
    }
    fn count(&mut self, n: usize, cap: usize) -> Result<(), Unavailable> {
        self.entries = self.entries.checked_add(n).ok_or(Unavailable::Limit)?;
        if n > cap || self.entries > 4096 {
            return Err(Unavailable::Limit);
        }
        self.u32(u32::try_from(n).map_err(|_| Unavailable::Limit)?)
    }
    fn text(&mut self, s: &str) -> Result<(), Unavailable> {
        crate::validation::text(s)?;
        self.u32(s.len() as u32)?;
        self.put(s.as_bytes())
    }
    fn modifiers(&mut self, values: &[Modifier]) -> Result<(), Unavailable> {
        self.count(values.len(), 64)?;
        for v in values {
            self.text(&v.id)?;
            self.u8(match v.kind {
                ModifierKind::Flat => 1,
                ModifierKind::Percent => 2,
                ModifierKind::Multiplier => 3,
            })?;
            self.f(v.value)?;
        }
        Ok(())
    }
}
fn war(w: &mut Writer, v: &WarFacts) -> Result<(), Unavailable> {
    w.f(v.weariness)?;
    w.f(v.minimum)?;
    w.bool(v.already_ended)?;
    w.bool(v.current_action)?;
    w.count(v.existing.len(), 256)?;
    for c in &v.existing {
        w.u8(match c.class {
            crate::ConcernClass::WarWeariness => 1,
            crate::ConcernClass::Other => 2,
        })?;
        w.bool(c.ended)?;
    }
    let p = &v.priority;
    for f in [
        p.alignment,
        p.max_alignment,
        p.positive_trait,
        p.negative_trait,
    ] {
        w.f(f)?;
    }
    w.count(p.traits.len(), 64)?;
    let mut traits: Vec<_> = p.traits.iter().collect();
    traits.sort();
    for t in traits {
        w.text(t)?;
    }
    w.bool(p.faction_multiplier.is_some())?;
    if let Some(f) = p.faction_multiplier {
        w.f(f)?;
    }
    w.count(p.tags.len(), 64)?;
    for t in &p.tags {
        w.text(t)?;
    }
    w.modifiers(&p.existing)
}
fn peace(w: &mut Writer, v: &PeaceFacts) -> Result<(), Unavailable> {
    w.text(&v.faction)?;
    w.text(&v.enemy)?;
    w.bool(v.enemy_is_player)?;
    w.bool(v.treaty_relation)?;
    for f in [
        v.own_weariness,
        v.enemy_weariness,
        v.own_events,
        v.enemy_events,
    ] {
        w.f(f)?;
    }
    let g = &v.gates;
    for b in [g.has_enemies, g.pirate, g.allow_pirate] {
        w.bool(b)?;
    }
    w.f(g.days)?;
    w.f(g.minimum_interval)?;
    for b in [
        g.recent_war,
        g.can_ceasefire,
        g.commissioned,
        g.offensive_blocked,
        g.offensive_facts_provided,
    ] {
        w.bool(b)?;
    }
    let r = &v.rules;
    for f in [r.minimum, r.divisor, r.divisor_per_level] {
        w.f(f)?;
    }
    w.u32(r.player_level)?;
    for f in [
        r.event_multiplier,
        r.treaty_chance,
        r.ceasefire_reduction,
        r.treaty_reduction,
    ] {
        w.f(f)?;
    }
    w.count(v.draws.len(), 2)?;
    for (ordinal, d) in v.draws.iter().enumerate() {
        w.u32(ordinal as u32)?;
        w.text(&d.purpose)?;
        w.put(&d.value.bits().to_le_bytes())?;
    }
    Ok(())
}
