use super::*;
use crate::{
    PeerError,
    receipt::{ReceiptPhase, ReceiptStatus, SourceMinima},
};
use nf_identity::private_storage::PrivateVault;
/// Explicit initial registration. All64 names must be absent; partial failure is not repaired.
pub fn initialize_book(
    vault: &PrivateVault,
    originals: &[(u8, BookRecord)],
) -> Result<BookCatalog, PeerError> {
    if originals.is_empty() || originals.len() > 8 {
        return Err(PeerError::Limit);
    }
    for (index, (slot, record)) in originals.iter().enumerate() {
        if *slot >= 8
            || record.generation != 0
            || record.previous != [0; 32]
            || record.phase != BookPhase::Unobserved
            || record.minimum.membership_revision < record.original.source().minimum_membership
            || originals[..index]
                .iter()
                .any(|(old, r)| old == slot || r.original.request() == record.original.request())
        {
            return Err(PeerError::Malformed);
        }
        record.validate()?;
    }
    recover_book(vault, &[])?;
    let mut anchors = Vec::with_capacity(originals.len());
    for (slot, record) in originals {
        vault
            .create_private_blob(&inventory::name(*slot, 0), &encode_book(record)?)
            .map_err(|_| PeerError::Storage)?;
        anchors.push(BookAnchor::from_record(*slot, record)?);
    }
    recover_book(vault, &anchors)
}
/// Appends only checked receipt metadata after a fresh complete inventory pass.
/// The caller must retain returned heads independently to claim rollback protection.
pub fn append_receipt(
    vault: &PrivateVault,
    anchors: &[BookAnchor],
    slot: u8,
    status: &ReceiptStatus,
    external: SourceMinima,
) -> Result<BookCatalog, PeerError> {
    let catalog = recover_book(vault, anchors)?;
    let old = catalog.head(slot)?;
    validate_status(old, status, external)?;
    let minimum = maximum(maximum(old.minimum, external), status.current);
    let phase = BookPhase::Receipt(status.phase);
    if minimum == old.minimum && phase == old.phase {
        return Ok(catalog);
    }
    if old.generation == 7 {
        return Err(PeerError::Backpressure);
    }
    let next = BookRecord {
        generation: old.generation + 1,
        previous: old.digest()?,
        original: old.original.clone(),
        ruleset: old.ruleset,
        content: old.content,
        minimum,
        phase,
    };
    next.follows(old)?;
    #[cfg(test)]
    super::collision_test::before_create(vault, &inventory::name(slot, next.generation))?;
    vault
        .create_private_blob(
            &inventory::name(slot, next.generation),
            &encode_book(&next)?,
        )
        .map_err(|_| PeerError::Storage)?;
    let mut updated = catalog.anchors()?;
    let anchor = updated
        .iter_mut()
        .find(|a| a.slot == slot)
        .ok_or(PeerError::Storage)?;
    *anchor = BookAnchor::from_record(slot, &next)?;
    recover_book(vault, &updated)
}
pub(super) fn maximum(a: SourceMinima, b: SourceMinima) -> SourceMinima {
    SourceMinima {
        event: a.event.max(b.event),
        store_revision: a.store_revision.max(b.store_revision),
        membership_revision: a.membership_revision.max(b.membership_revision),
    }
}
pub fn validate_status(
    old: &BookRecord,
    s: &ReceiptStatus,
    external: SourceMinima,
) -> Result<(), PeerError> {
    let o = old.original.original();
    if s.request != o.request_id
        || s.account != o.account_id
        || s.device != o.device_id
        || !s.current.admits(maximum(old.minimum, external))
    {
        return Err(PeerError::Unauthorized);
    }
    match s.phase {
        ReceiptPhase::Unknown if s.operation.as_bytes() == &[0; 16] && s.binding == [0; 32] => {}
        ReceiptPhase::Pending | ReceiptPhase::Rejected { .. } | ReceiptPhase::Committed { .. }
            if s.operation == old.original.operation()
                && s.binding == old.original.binding_digest() => {}
        _ => return Err(PeerError::Unauthorized),
    }
    if let ReceiptPhase::Rejected { sequence } | ReceiptPhase::Committed { sequence } = s.phase
        && (sequence.0 == 0 || sequence > s.current.event)
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
