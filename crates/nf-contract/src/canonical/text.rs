//! Unicode 13 assigned scalars (excluding surrogates/noncharacters) and standard NFC.
//! This prevents newer normalizers admitting forms older supported JVMs cannot evaluate.
use core::cmp::Ordering;
use unicode_normalization::is_nfc;
#[rustfmt::skip]
#[path = "text_ranges.rs"]
mod text_ranges;

pub fn is_admitted_scalar(scalar: char) -> bool {
    let scalar = u32::from(scalar);
    text_ranges::RANGES
        .binary_search_by(|&(start, end)| {
            if scalar < start {
                Ordering::Greater
            } else if scalar > end {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}
pub fn is_canonical_text(text: &str) -> bool {
    text.chars().all(is_admitted_scalar) && is_nfc(text)
}
