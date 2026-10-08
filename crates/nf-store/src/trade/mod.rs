//! Explicit profile6 authority-vault trade seam; no campaign/native or wire activation.
mod auth;
mod error;
mod model;
mod owner;
pub(crate) mod policy;
mod query;
pub(crate) mod schema;
pub use error::TradeStoreError;
pub use model::{KnownTradeFrontiers, TradeChallenge};
pub use owner::TradeStore;
