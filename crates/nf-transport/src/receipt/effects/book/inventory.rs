use super::*;
use crate::PeerError;
use nf_identity::{model::IdentityError, private_storage::PrivateVault};
/// Every pass probes all64 fixed names. No partial catalog escapes a failed pass.
pub fn recover_book(
    vault: &PrivateVault,
    anchors: &[BookAnchor],
) -> Result<BookCatalog, PeerError> {
    super::recover_book_detailed(vault, anchors).map_err(BookFailure::peer_error)
}
pub(crate) fn recover_book_detailed(
    vault: &PrivateVault,
    anchors: &[BookAnchor],
) -> BookResult<BookCatalog> {
    if anchors.len() > 8 {
        return Err(PeerError::Limit.into());
    }
    let mut configured: [Option<&BookAnchor>; 8] = [None; 8];
    for (index, anchor) in anchors.iter().enumerate() {
        if anchor.slot >= 8 || anchor.generation >= 8 || anchor.digest == [0; 32] {
            return Err(PeerError::Malformed.into());
        }
        if configured[anchor.slot as usize].is_some()
            || anchors[..index]
                .iter()
                .any(|old| old.original.request() == anchor.original.request())
        {
            return Err(PeerError::Malformed.into());
        }
        configured[anchor.slot as usize] = Some(anchor);
    }
    let names: [String; 64] =
        std::array::from_fn(|index| name((index / 8) as u8, (index % 8) as u8));
    let borrowed: [&str; 64] = std::array::from_fn(|index| names[index].as_str());
    let mut heads: [Option<BookRecord>; 8] = std::array::from_fn(|_| None);
    let mut anchored = [false; 8];
    let mut failure: Option<PeerError> = None;
    let result = vault.scan_optional_private_blobs_detailed(&borrowed, |index, payload| {
        let Some(payload) = payload else {
            return Ok(());
        };
        let slot = index / 8;
        let generation = (index % 8) as u8;
        let admitted: Result<(), PeerError> = (|| {
            let anchor = configured[slot].ok_or(PeerError::Storage)?;
            let mut record = decode_book(payload)?;
            if record.generation != generation || !anchor.matches(&record) {
                return Err(PeerError::Replay);
            }
            if let Some(old) = &heads[slot] {
                record.follows(old)?;
            } else if generation != 0 {
                return Err(PeerError::Replay);
            }
            if generation == anchor.generation {
                if record.digest()? != anchor.digest {
                    return Err(PeerError::Replay);
                }
                anchored[slot] = true;
            }
            record.original = anchor.original.clone();
            heads[slot] = Some(record);
            Ok(())
        })();
        if let Err(error) = admitted {
            failure = Some(error);
            return Err(IdentityError::Malformed);
        }
        Ok(())
    });
    if let Some(error) = failure {
        return Err(error.into());
    }
    result.map_err(BookFailure::storage)?;
    for anchor in anchors {
        let slot = anchor.slot as usize;
        let Some(head) = &heads[slot] else {
            return Err(PeerError::Storage.into());
        };
        if !anchored[slot]
            || !head.minimum.admits(anchor.minimum)
            || head.minimum.membership_revision < anchor.original.source().minimum_membership
        {
            return Err(PeerError::Replay.into());
        }
    }
    Ok(BookCatalog { heads })
}
pub(super) fn name(slot: u8, generation: u8) -> String {
    format!("receipt-r{slot}-g{generation}")
}
