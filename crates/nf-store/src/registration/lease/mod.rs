//! Explicit headless lease owner. No game bridge or economic capability is granted.
mod auth;
mod codec;
mod model;
mod owner;
mod replay;
mod schema;
mod write;

pub use auth::{AuthTicket, IssuedChallenge, ProofAttempt};
pub use model::{
    AdmissionMode, AdmissionPolicy, AdmitLease, ClientSessionId, KnownAdmissionFrontier,
    LeaseError, LeaseGranted,
};
pub use owner::LeaseStore;
