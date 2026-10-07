use super::budget::{Sink, Writer};
use crate::Unavailable;
use alloc::vec::Vec;
use nf_nex_boundary::*;
pub(super) fn definition<S: Sink>(w: &mut Writer<S>, d: &Definition) -> Result<(), Unavailable> {
    let Definition {
        id,
        class_path,
        module,
    } = d;
    w.text(id)?;
    w.text(class_path)?;
    w.byte(match module {
        Module::Diplomatic => 1,
        Module::Economic => 2,
        Module::Military => 3,
        Module::Executive => 4,
    })
}
pub(super) fn faction<S: Sink>(
    w: &mut Writer<S>,
    f: &FactionView,
    canonical: bool,
) -> Result<(), Unavailable> {
    let FactionView {
        id,
        live,
        traits,
        diplomatic_alignment,
        priority_multipliers,
    } = f;
    w.text(id)?;
    w.boolean(*live)?;
    w.count(traits.len(), 64)?;
    if canonical {
        let mut sorted: Vec<_> = traits.iter().collect();
        sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        for t in sorted {
            w.text(t)?;
        }
    } else {
        for t in traits {
            w.text(t)?;
        }
    }
    w.float(*diplomatic_alignment)?;
    w.count(priority_multipliers.len(), 64)?;
    for (id, value) in priority_multipliers {
        w.text(id)?;
        w.float(*value)?;
    }
    Ok(())
}
pub(super) fn relation<S: Sink>(w: &mut Writer<S>, r: &RelationView) -> Result<(), Unavailable> {
    let RelationView {
        from,
        to,
        relationship,
        disposition,
        hostile,
    } = r;
    w.text(from)?;
    w.text(to)?;
    w.float(*relationship)?;
    w.boolean(disposition.is_some())?;
    if let Some(v) = disposition {
        w.float(*v)?;
    }
    w.boolean(*hostile)
}
pub(super) fn strength<S: Sink>(w: &mut Writer<S>, s: &StrengthView) -> Result<(), Unavailable> {
    let StrengthView {
        faction,
        markets,
        fleet_strength,
    } = s;
    w.text(faction)?;
    w.count(markets.len(), 256)?;
    for m in markets {
        let MarketStrength { id, size, strength } = m;
        w.text(id)?;
        w.byte(*size)?;
        w.float(*strength)?;
    }
    w.float(*fleet_strength)
}
pub(super) fn weariness<S: Sink>(w: &mut Writer<S>, v: &WearinessView) -> Result<(), Unavailable> {
    let WearinessView {
        manager_present,
        map_entry_present,
        raw,
        adjusted,
        enemies,
        filter,
        minimum_for_peace,
    } = v;
    w.boolean(*manager_present)?;
    w.boolean(*map_entry_present)?;
    w.float(*raw)?;
    w.float(*adjusted)?;
    w.count(enemies.len(), 256)?;
    for e in enemies {
        w.text(e)?;
    }
    match filter {
        EnemyFilter::AdjustedGetter { allow_pirates } => {
            w.byte(1)?;
            w.boolean(*allow_pirates)?;
        }
    }
    w.float(*minimum_for_peace)
}
pub(super) fn concern_config<S: Sink>(
    w: &mut Writer<S>,
    c: &ConcernConfig,
) -> Result<(), Unavailable> {
    let ConcernConfig {
        definition: d,
        enabled,
        no_auto_generate,
        tags,
        cooldown_multiplier,
        anti_repetition_multiplier,
    } = c;
    definition(w, d)?;
    w.boolean(*enabled)?;
    w.boolean(*no_auto_generate)?;
    w.count(tags.len(), 64)?;
    for t in tags {
        w.text(t)?;
    }
    w.float(*cooldown_multiplier)?;
    w.float(*anti_repetition_multiplier)
}
pub(super) fn action_config<S: Sink>(
    w: &mut Writer<S>,
    c: &ActionConfig,
) -> Result<(), Unavailable> {
    let ActionConfig {
        definition: d,
        enabled,
        shim,
        tags,
        chance,
        cooldown,
        anti_repetition,
        repetition_id,
    } = c;
    definition(w, d)?;
    w.boolean(*enabled)?;
    w.boolean(*shim)?;
    w.count(tags.len(), 64)?;
    for t in tags {
        w.text(t)?;
    }
    w.float(*chance)?;
    w.float(*cooldown)?;
    w.float(*anti_repetition)?;
    w.optional_text(repetition_id.as_deref())
}
pub(super) fn action<S: Sink>(w: &mut Writer<S>, a: &ActionView) -> Result<(), Unavailable> {
    let ActionView {
        instance,
        definition: d,
        status,
        ended,
        meetings_since_ended,
    } = a;
    w.raw(instance.as_bytes())?;
    definition(w, d)?;
    w.byte(match status {
        ActionStatus::Starting => 1,
        ActionStatus::InProgress => 2,
        ActionStatus::Success => 3,
        ActionStatus::Failure => 4,
        ActionStatus::Cancelled => 5,
    })?;
    w.boolean(*ended)?;
    w.u32(*meetings_since_ended)
}
pub(super) fn concern<S: Sink>(w: &mut Writer<S>, c: &ConcernView) -> Result<(), Unavailable> {
    let ConcernView {
        instance,
        definition: d,
        ended,
        cooldown,
        action: a,
        target_faction,
        market_id,
        priority_base,
        priority,
    } = c;
    w.raw(instance.as_bytes())?;
    definition(w, d)?;
    w.boolean(*ended)?;
    w.float(*cooldown)?;
    w.boolean(a.is_some())?;
    if let Some(a) = a {
        action(w, a)?;
    }
    w.optional_text(target_faction.as_deref())?;
    w.optional_text(market_id.as_deref())?;
    w.float(*priority_base)?;
    w.count(priority.len(), 64)?;
    for m in priority {
        let Modifier { id, kind, value } = m;
        w.text(id)?;
        w.byte(match kind {
            ModifierKind::Flat => 1,
            ModifierKind::Percent => 2,
            ModifierKind::Multiplier => 3,
        })?;
        w.float(*value)?;
    }
    Ok(())
}
pub(super) fn timers<S: Sink>(w: &mut Writer<S>, t: &TimerView) -> Result<(), Unavailable> {
    let TimerView {
        meeting,
        advance_days,
        intervals,
        draws,
    } = t;
    w.u64(*meeting)?;
    w.float(*advance_days)?;
    w.count(intervals.len(), 64)?;
    for i in intervals {
        let IntervalView {
            id,
            elapsed,
            minimum,
            maximum,
        } = i;
        w.text(id)?;
        w.float(*elapsed)?;
        w.float(*minimum)?;
        w.float(*maximum)?;
    }
    w.count(draws.len(), 4096)?;
    for d in draws {
        let RandomDraw {
            purpose,
            ordinal,
            value,
        } = d;
        w.text(purpose)?;
        w.u32(*ordinal)?;
        w.double(*value)?;
    }
    Ok(())
}
pub(super) fn extensions<S: Sink>(
    w: &mut Writer<S>,
    e: &ExtensionInventory,
) -> Result<(), Unavailable> {
    let ExtensionInventory {
        complete,
        listeners,
        direct_calls,
        other_definitions,
    } = e;
    w.boolean(*complete)?;
    for list in [listeners, direct_calls] {
        w.count(list.len(), 256)?;
        for r in list {
            let Registration { class_path, origin } = r;
            w.text(class_path)?;
            w.text(origin)?;
        }
    }
    w.count(other_definitions.len(), 256)?;
    for d in other_definitions {
        definition(w, d)?;
    }
    Ok(())
}
