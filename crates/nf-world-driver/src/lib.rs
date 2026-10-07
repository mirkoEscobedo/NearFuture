#![no_std]
//! Pure scheduling hints. No pacing observation or hint authorizes a durable mutation.
mod pacing;
pub use pacing::{DueHint, Pacer, PacingError};
