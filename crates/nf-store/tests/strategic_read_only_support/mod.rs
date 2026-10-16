//! Real finite public fixture evidence. No deleting destructor or child process.
mod custody;
mod permissions;
mod projection;
mod sql;
pub use custody::Case;
pub use projection::{EXPECTED, EXPECTED_SHA256, assert_private_fields_absent, projection};
pub use sql::SqlImage;
pub fn hash(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}
