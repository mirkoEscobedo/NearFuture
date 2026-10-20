//! Passive returned failure details. Existing effects and public error classes stay unchanged.
#[path = "private_diagnostics/helper.rs"]
mod helper;
#[path = "private_diagnostics/model.rs"]
mod model;
#[cfg(windows)]
pub(crate) use helper::exit_stage as helper_exit_stage;
pub use helper::*;
pub use model::*;
#[path = "private_diagnostics/access.rs"]
mod access;
pub(crate) use access::check as access_cause;
#[path = "private_diagnostics/vault.rs"]
mod vault;
