//! Passive returned failure details. Existing effects and public error classes stay unchanged.
#[path = "private_diagnostics/helper.rs"]
mod helper;
#[path = "private_diagnostics/identity_create.rs"]
mod identity_create;
#[path = "private_diagnostics/model.rs"]
mod model;
pub use helper::*;
pub use model::*;

#[cfg(windows)]
pub(crate) use helper::exit_stage as helper_exit_stage;

#[path = "private_diagnostics/access.rs"]
mod access;
pub(crate) use access::check as access_cause;

#[path = "private_diagnostics/blob_create.rs"]
mod blob_create;

#[path = "private_diagnostics/vault.rs"]
mod vault;
