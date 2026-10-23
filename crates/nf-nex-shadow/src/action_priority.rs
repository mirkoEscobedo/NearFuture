//! Copied MakePeaceAction additional weariness multiplier; no game or superclass calls.
//! Nexerelin a669f4d0740e95a4acbb6b894dde09ade67aa754, MIT copyright2015 L.J. "Histidine" Lim.
//! Primary MakePeaceAction SHA256 d5d43828ea5391cc0bcc4079bc63594875aa77d0ec5ab903d95883fa7238e20d.
use crate::{
    Unavailable,
    validation::{f, finite},
};
use alloc::string::ToString;
use nf_nex_boundary::{FloatBits, Modifier, ModifierKind};

/// Diagnostic boundary: finite supplied inputs, positive minimum, finite output.
/// These limits are not Nex refusal rules. No inherited/aggregate priority or identity is inferred.
/// Trace the source-defined cap of five before validating the finite output representation.
pub fn make_peace_weariness_modifier(
    adjusted_weariness: FloatBits,
    minimum_for_peace: FloatBits,
) -> Result<Modifier, Unavailable> {
    let minimum = f(minimum_for_peace);
    if minimum <= 0.0 {
        return Err(Unavailable::Unsupported);
    }
    let ratio = f(adjusted_weariness) / minimum;
    Ok(Modifier {
        id: "weariness".to_string(),
        kind: ModifierKind::Multiplier,
        value: finite(if ratio > 5.0 { 5.0 } else { ratio })?,
    })
}
