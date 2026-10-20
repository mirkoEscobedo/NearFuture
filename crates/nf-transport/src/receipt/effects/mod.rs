//! Sole-owner receipt effects. Persisted observations never grant protocol authority.
pub mod book;
mod model;
mod repository;
pub(crate) use model::{CurrentRead, RemotePrincipal};
pub use model::{ProjectionRead, ReceiptSelector};
pub use repository::{ReceiptConfig, ReceiptRepo};
mod selector;
mod trusted_commit;
pub use trusted_commit::TrustedPreparedCommit;
mod operation;
pub use operation::AdmittedReceipt;
pub(crate) use operation::{ReceiptOperationClient, ReceiptOperationServer};
mod activation;
mod handshake;
mod observation;
pub(crate) use handshake::{ReceiptClientHandshake, ReceiptServerHandshake};
mod revocation;

mod owner_codec;
pub use owner_codec::{ReceiptOwnerCodec, ReceiptRequest};

mod network;
pub(crate) use network::build_receipt_swarm;
pub use network::{ReceiptBehaviour, ReceiptBehaviourEvent};

mod inbound;
use inbound::InboundDelivery;
mod handshake_delivery;

mod handshake_custody;

mod lane;
pub use lane::{ReceiptLane, ReceiptLaneEvent};

mod completion;
pub use completion::ReceiptCompletion;
pub(crate) use completion::ReceiptQueryBinding;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod read_delay;
