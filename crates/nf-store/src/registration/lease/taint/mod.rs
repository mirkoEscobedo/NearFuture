//! Authenticated headless self-reports only; no detector or economic authority is certified.
mod auth;
mod codec;
mod model;
mod owner;
mod replay;
mod schema;
mod write;

pub use auth::{AuthTicket, IssuedChallenge, ProofAttempt};
pub use model::{
    AuthorityLineageId, CampaignLineage, KnownTaintFrontier, MarkProhibitedManifest, TaintCause,
    TaintError, TaintMode, TaintPolicy, TaintRecorded,
};
pub use owner::TaintStore;
