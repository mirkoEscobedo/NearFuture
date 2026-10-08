//! Separate local Chat outbox. Pending is local; delivery requires a later verified receiver receipt.
mod codec;
mod model;
mod owner;
mod schema;
pub use model::{OutboxProfile, OutgoingEntry, OutgoingState};
pub use owner::ClientOutbox;
mod receipt;
mod write;
pub use receipt::{ChatDeliveryReceipt, SignedChatReceipt};
