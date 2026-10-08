//! Headless branch registration only; no lease, asset or native capability is granted.
mod auth;
mod codec;
mod membership;
mod model;
mod owner;
mod replay;
mod schema;
mod write;

pub use auth::{AuthTicket, IssuedChallenge, ProofAttempt};
pub use model::{
    AllowedBranch, CampaignBinding, KnownRegistrationFrontier, RegisterBranch, RegisteredBranch,
    RegistrationError, RegistrationMode, RegistrationPolicy,
};
pub use owner::BranchRegistrar;

pub mod lease;
