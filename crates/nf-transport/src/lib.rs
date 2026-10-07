//! Explicit peer transport effects with closed Rust-only records. Never called from a game frame.
#[cfg(test)]
extern crate self as nf_transport;

pub mod bulk;
mod error;
pub use error::PeerError;
pub mod auth;
pub mod budget;
pub mod framing;
pub mod identity;
pub mod mux;
pub mod network;
pub mod node;
pub mod notification;
pub mod query;
pub mod receipt;
pub mod records;
pub mod session;

#[path = "notification/effects/mod.rs"]
pub mod notification_effects;
#[path = "receipt/effects/mod.rs"]
pub mod receipt_effects;

#[path = "portal/config/mod.rs"]
pub mod portal_config;

#[path = "portal/owner/mod.rs"]
pub mod portal;

pub mod sync;
