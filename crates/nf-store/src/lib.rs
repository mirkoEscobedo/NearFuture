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

// The exact maintained connection shell is shared without changing miniature APIs.
mod strategic_reader;
#[path = "miniature/reader_connection.rs"]
mod strategic_reader_connection;
pub use strategic_reader::{
    StrategicReader, StrategicSupportAssignmentV1, StrategicSupportCountsV1,
    StrategicSupportImplementation, StrategicSupportManifestV1, StrategicSupportObservationV1,
    StrategicSupportProtectionV1, StrategicSupportRulesetBinding, StrategicSupportUnavailable,
    SupportProtection,
};
