//! Fixed implementation identity of the private miniature profile.
//! This identifies the LF-normalized owned source bundle, not a compiled binary.
use super::generated_pin;
/// The schema2 codec/reducer has one compiled implementation identity; callers cannot select it.
pub const MINIATURE_IMPLEMENTATION_HASH: [u8; 32] = generated_pin::SOURCE_HASH;
