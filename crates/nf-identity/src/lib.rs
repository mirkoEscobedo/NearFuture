//! Account/device admission policy and explicitly separated private-key effects.
pub mod authorization;
pub mod codec;
mod key_staging;
pub mod keys;
pub mod membership;
pub mod model;
#[cfg(windows)]
mod owned_process;
pub mod persistence;
mod private_blob;
#[cfg(windows)]
mod private_blob_acl;
mod private_blob_scan;
mod private_diagnostics;
pub mod private_storage;
pub mod rotation;
pub mod signing;

#[cfg(windows)]
mod private_acl_session;
mod protected_blob_scope;
