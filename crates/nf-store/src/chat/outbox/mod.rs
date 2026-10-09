//! Separate local Chat outbox. Pending is local; delivery requires a later verified receiver receipt.
mod codec;
mod model;
mod owner;
mod schema;
pub use model::{OutboxProfile, OutgoingEntry, OutgoingState};
pub use owner::ClientOutbox;
mod receipt;
mod write;
pub(super) use receipt::sign_delivery;
pub use receipt::{ChatDeliveryReceipt, SignedChatReceipt};
