//! Trusted local managed ownership and full NFST4 continuation.
//!
//! The catalog gate, actual selected connection and fresh device proofs admit commands.
//! Immutable stamps, prepared files and pure cached getters are data, never authority.
//! Operational executable qualification and a nonrollback catalog custody service are
//! separate deployment obligations; this library does not certify either.
mod active_origin;
mod active_plan;
mod active_read;
mod admission;
mod adoption;
mod auth;
mod backup;
mod book;
mod bytes;
mod cancellation;
mod catalog;
mod catalog_action;
mod catalog_command;
mod catalog_observation;
mod catalog_payload;
mod catalog_read;
mod catalog_reduce;
mod catalog_rules;
mod catalog_state;
mod challenge;
mod coordinator;
mod delta;
mod effect;
mod engine;
mod error;
mod failure;
mod file_io;
mod foot;
mod input;
mod journal;
mod maps;
mod mirror_rows;
mod mutation;
mod observations;
mod owner;
mod owner_ops;
mod page_room;
mod pending;
mod pending_open;
mod pending_verify;
mod prepare;
mod prepared;
mod proof;
mod prospective;
mod readonly;
mod receipt;
mod reconcile;
mod registry;
mod retire;
mod retired_read;
mod retirement;
mod rows;
mod schema;
mod sealed;
mod selection;
mod settlement;
mod source_owner;
mod source_plan;
mod stamp;
mod state;
mod summary;
mod upgrade;
mod world_file;
mod world_plan;

pub use adoption::AdoptionDraft;
pub use cancellation::{AdministrativeCancellation, AdministrativeCancellationKind};
pub use catalog_observation::{ManagedCatalogObservation, RegistryObservation};
pub use challenge::{Challenge, PrincipalProof};
pub use coordinator::{LocalCoordinator, UnclaimedCoordinator};
pub use error::ManagedError;
pub use failure::{
    AdoptionFailure, AdoptionOutcome, PrecommitFailure, SettlementFailure, UpgradeOutcome,
};
pub use foot::StorageFoot;
pub use input::{AdoptionInput, BinaryEvidence, FilePin, PrepareInput, SettlementInput};
pub use observations::{ManagedOutboxRecord, ManagedRequestStatus, ManagedSnapshot};
pub use owner::{ActiveOwner, CurrentOwner, SourceOwner};
pub use prepared::PreparedActiveUpgrade;
pub use receipt::{Receipt as OperationReceipt, ReceiptResult};
pub use reconcile::{ReconciledStatus, ReconciliationHandle};
pub use schema::{active_schema_digest, catalog_schema_digest, retired_schema_digest};
pub use sealed::ManagedDrainedSource;
pub use selection::SourceFenced;
pub use stamp::{CatalogFrontier, OwnerStamp};
pub use state::{PendingPolicy, QuiescePolicy};
