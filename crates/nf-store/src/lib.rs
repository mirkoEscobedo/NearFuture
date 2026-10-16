//! Local single-authority SQLite storage. No success/outbox publication precedes durable commit.
mod codec;
mod error;
mod identity;
mod mirrors;
mod model;
mod persistence;
mod recovery;
mod schema;
mod store;
mod transitions;
pub use error::StoreError;
pub use model::{
    Accepted, BoundRequestStatus, Boundary, DurableAck, KnownFrontiers, OutboxRecord,
    PrincipalDevice, QuorumCommitPort, RequestStatus, Reservation,
};
pub use store::{ImmutableSnapshot, Store};

/// Guarded private miniature profile; schema1 storage APIs remain separate.
pub mod miniature;

pub mod supplies;

pub mod registration;
pub mod trade;

pub mod migration;

/// Inactive config-only artifacts, never ordinary Store or activation admission.
pub mod staging;

/// Replayable inactive NFST3 upgrades; no owner selection or writer admission.
pub mod upgrade;

/// Authenticated local catalog ownership and full active-format continuation.
pub mod managed;
