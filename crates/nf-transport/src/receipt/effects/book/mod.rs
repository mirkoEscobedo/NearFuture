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
