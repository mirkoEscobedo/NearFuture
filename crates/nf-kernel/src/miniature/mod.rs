//! Closed NF miniature profile. These pure values do not establish authentication or durability.
mod model;
pub use model::*;
mod codec;
mod world;
pub use codec::*;
mod frontier;
pub use frontier::admit_miniature;
mod reducer;
pub use reducer::{replay_miniature, settle_miniature};
mod cancel;
pub use cancel::cancel_miniature;
mod profile;
pub use profile::MINIATURE_IMPLEMENTATION_HASH;
mod generated_pin;
