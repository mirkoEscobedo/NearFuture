//! Pure, adapted-source MakePeaceAction eligibility over supplied facts only.
//! Adapted MakePeaceAction.canUse from Nexerelin v0.12.2c.
//! Copyright (c) 2015 L.J. "Histidine" Lim; complete MIT notice: LICENSE-NEX-MIT.txt.
//! Source commit: a669f4d0740e95a4acbb6b894dde09ade67aa754.
//! Source Git blob: bb581ac2671eb76e3a97c537ce059c0c130baa11.
//! Source SHA-256: d5d43828ea5391cc0bcc4079bc63594875aa77d0ec5ab903d95883fa7238e20d.
//! Game/concern getters become supplied booleans; an absent target maps to None.
//! The overridden base is not invoked; eligibility returns no live effects.
use crate::Unavailable;

/// Explicit copied facts; no live-state, identity-binding or authority certification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MakePeaceEligibility {
    pub diplomacy_enabled: bool,
    pub concern_can_make_peace: bool,
    /// None explicitly means no target, never unknown live state.
    pub target_hostile: Option<bool>,
    pub faction_diplomacy_disabled: bool,
}

/// Adapted MakePeaceAction.canUse override from pinned Nexerelin a669f4d; diagnostics only.
pub fn make_peace_action_eligible(facts: &MakePeaceEligibility) -> Result<bool, Unavailable> {
    if !facts.diplomacy_enabled {
        return Ok(false);
    }
    if !facts.concern_can_make_peace {
        return Ok(false);
    }
    if facts.target_hostile == Some(false) {
        return Ok(false);
    }
    Ok(!facts.faction_diplomacy_disabled)
}
