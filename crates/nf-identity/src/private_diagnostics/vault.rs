use super::{PrivateCause, PrivateFailure, PrivateOperation, PrivateResult, PrivateStage};
use crate::private_storage::PrivateVault;
use std::{fs, path::Path};

impl PrivateVault {
    /// Performs creation once and preserves bounded evidence from the failed effect.
    pub fn create_detailed(path: &Path, game_save_root: &Path) -> PrivateResult<Self> {
        fs::create_dir(path).map_err(|error| {
            PrivateFailure::io(
                PrivateOperation::VaultCreate,
                PrivateStage::CreateDirectory,
                error,
            )
        })?;
        super::access_cause(path, true).map_err(|cause| {
            PrivateFailure::with_cause(
                PrivateOperation::VaultCreate,
                PrivateStage::RootAccess,
                cause,
            )
        })?;
        Self::open_detailed(path, game_save_root)
    }
    /// Preserves private-root and game-save exclusion checks without replacement state.
    pub fn open_detailed(path: &Path, game_save_root: &Path) -> PrivateResult<Self> {
        super::access_cause(path, false).map_err(|cause| {
            PrivateFailure::with_cause(PrivateOperation::VaultOpen, PrivateStage::RootAccess, cause)
        })?;
        let root = fs::canonicalize(path).map_err(|error| {
            PrivateFailure::io(
                PrivateOperation::VaultOpen,
                PrivateStage::RootCanonicalize,
                error,
            )
        })?;
        let saves = fs::canonicalize(game_save_root).map_err(|error| {
            PrivateFailure::io(
                PrivateOperation::VaultOpen,
                PrivateStage::SaveRootCanonicalize,
                error,
            )
        })?;
        if root.starts_with(&saves) || !root.is_dir() {
            return Err(PrivateFailure::with_cause(
                PrivateOperation::VaultOpen,
                PrivateStage::RootRelation,
                PrivateCause::Refused,
            ));
        }
        Ok(Self { root })
    }
}
