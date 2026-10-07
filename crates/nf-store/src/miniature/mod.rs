//! Local guarded authority for the private miniature SQLite profile.
mod error;
mod model;
mod owner;
pub use error::MiniatureStoreError;
pub use model::*;
pub use owner::MiniatureStore;
mod auth;
mod bootstrap;
mod codec;
mod schema;
pub use auth::{AuthTicket, IssuedChallenge, ProofAttempt};
mod membership;
mod mirrors;
mod recovery;
mod state;
pub use bootstrap::bootstrap_binding_preimage;
mod activity;
mod advancement;
mod authority;
mod cancellation;
mod challenges;
mod economic;
mod maintenance;
mod persistence;
mod resumption;
mod transitions;

#[cfg(test)]
mod tests;
