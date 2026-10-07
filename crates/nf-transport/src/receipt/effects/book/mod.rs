mod codec;
mod model;
pub use codec::{decode_book, encode_book};
pub use model::*;
mod anchor;
mod inventory;
mod persistence;
mod reader;
pub use anchor::{BookAnchor, BookCatalog};
pub use inventory::recover_book;
pub use persistence::{append_receipt, initialize_book, validate_status};

#[cfg(test)]
mod collision_test;

mod diagnostic;
pub(crate) use diagnostic::{BookFailure, BookResult};
pub(crate) use inventory::recover_book_detailed;
#[cfg(test)]
mod diagnostic_tests;
pub(crate) use persistence::append_receipt_detailed;
pub(crate) use persistence::initialize_book_detailed;
