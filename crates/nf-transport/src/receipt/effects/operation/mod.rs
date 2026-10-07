mod client;
mod common;
mod model;
mod server;
pub(crate) use client::ReceiptOperationClient;
pub(super) use common::fresh;
pub use model::AdmittedReceipt;
pub(crate) use model::VerifiedReceipt;
pub(super) use model::{VerifiedResult, fresh_deadline};
pub(crate) use server::ReceiptOperationServer;

mod delivery;
mod outbound;

pub(super) use delivery::{Delivery, Origin};

#[cfg(test)]
mod delivery_tests;
