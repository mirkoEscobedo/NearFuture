use super::{
    budget::{Sink, Writer},
    records::*,
};
use crate::Unavailable;
use nf_nex_boundary::*;
pub(super) fn profile<S: Sink>(
    w: &mut Writer<S>,
    world: &NexWorld,
    canonical: bool,
) -> Result<(), Unavailable> {
    let Assessment {
        authority,
        runtime,
        findings,
    } = world.assessment();
    if !findings.is_empty() {
        return Err(Unavailable::Unsupported);
    }
    w.raw(b"NF-NEX-WORLD-1\0")?;
    w.raw(&1_u16.to_le_bytes())?;
    w.byte(match authority {
        Authority::ShadowOnly => 1,
    })?;
    w.byte(match runtime {
        RuntimeCertification::Unobserved => 1,
    })?;
    let Snapshot {
        provenance,
        subject_faction,
        factions,
        relations,
        strengths,
        weariness: v,
        priority_rules,
        concern_config: c,
        action_configs,
        concerns,
        timers: t,
        extensions: e,
    } = world.snapshot();
    provenance_record(w, provenance)?;
    w.text(subject_faction)?;
    w.count(factions.len(), 256)?;
    for f in factions {
        faction(w, f, canonical)?;
    }
    w.count(relations.len(), 4096)?;
    for r in relations {
        relation(w, r)?;
    }
    w.count(strengths.len(), 256)?;
    for s in strengths {
        strength(w, s)?;
    }
    weariness(w, v)?;
    let PriorityRules {
        max_alignment_modifier,
        positive_trait_multiplier,
        negative_trait_multiplier,
    } = priority_rules;
    w.float(*max_alignment_modifier)?;
    w.float(*positive_trait_multiplier)?;
    w.float(*negative_trait_multiplier)?;
    concern_config(w, c)?;
    w.count(action_configs.len(), 64)?;
    for a in action_configs {
        action_config(w, a)?;
    }
    w.count(concerns.len(), 256)?;
    for c in concerns {
        concern(w, c)?;
    }
    timers(w, t)?;
    extensions(w, e)
}
fn provenance_record<S: Sink>(w: &mut Writer<S>, p: &Provenance) -> Result<(), Unavailable> {
    let Provenance {
        source_commit,
        source_digest,
        runtime_digest,
        ruleset_digest,
        merged_config_digest,
        universe,
        history,
        runtime_session,
        frontier,
        observation,
    } = p;
    if source_commit != SOURCE_COMMIT
        || runtime_session.0 == 0
        || [
            source_digest,
            runtime_digest,
            ruleset_digest,
            merged_config_digest,
        ]
        .iter()
        .any(|d| **d == [0; 32])
    {
        return Err(Unavailable::MissingFact);
    }
    w.text(source_commit)?;
    for d in [
        source_digest,
        runtime_digest,
        ruleset_digest,
        merged_config_digest,
    ] {
        w.raw(d)?;
    }
    w.raw(universe.as_bytes())?;
    w.raw(history.as_bytes())?;
    w.u64(runtime_session.0)?;
    w.u64(frontier.0)?;
    w.byte(match observation {
        Observation::Synthetic => 1,
        Observation::CapturedUnverified => 2,
    })
}
