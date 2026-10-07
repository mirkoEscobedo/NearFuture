//! Passive denial evidence; retains no SQL text, paths or underlying error strings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactStage {
    Checkpoint,
    Vacuum,
    CheckpointDelete,
    CheckpointRowWrite,
    CheckpointCommit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactReason {
    AttachmentLimitZero,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactCause {
    Sqlite {
        primary: i32,
        extended: i32,
        reason: Option<CompactReason>,
    },
    Storage(crate::StoreError),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompactFailure {
    pub stage: CompactStage,
    pub cause: CompactCause,
}

pub(super) fn denied(
    stage: CompactStage,
    error: rusqlite::Error,
) -> super::error::SuppliesStoreError {
    let cause = match error {
        rusqlite::Error::SqliteFailure(detail, message) => {
            let extended = detail.extended_code;
            let primary = extended & 0xff;
            let reason = if primary == 1
                && message.as_deref() == Some("too many attached databases - max 0")
            {
                Some(CompactReason::AttachmentLimitZero)
            } else {
                None
            };
            CompactCause::Sqlite {
                primary,
                extended,
                reason,
            }
        }
        other => CompactCause::Storage(other.into()),
    };
    super::error::SuppliesStoreError::Compact(CompactFailure { stage, cause })
}
/// Rewrites only the verified replay-derived checkpoint, retaining every original record.
pub(super) fn checkpoint(
    connection: &rusqlite::Connection,
    state: &super::ledger::State,
) -> super::error::Result<()> {
    connection
        .execute("DELETE FROM supplies_balances", [])
        .map_err(|error| denied(CompactStage::CheckpointDelete, error))?;
    for ((owner, content, origin), balance) in &state.balances {
        super::ledger::conservation(*balance)?;
        connection
            .execute(
                "INSERT INTO supplies_balances VALUES(?1,?2,?3,?4)",
                rusqlite::params![
                    owner.as_bytes(),
                    content,
                    nf_kernel::supplies::origin_bytes(*origin),
                    super::ledger::balance_bytes(*balance)
                ],
            )
            .map_err(|error| denied(CompactStage::CheckpointRowWrite, error))?;
    }
    Ok(())
}
