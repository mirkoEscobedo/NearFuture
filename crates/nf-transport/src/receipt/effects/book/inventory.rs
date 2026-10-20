use super::*;
use crate::PeerError;
use nf_identity::{model::IdentityError, private_storage::PrivateVault};
/// Every pass probes all64 fixed names. No partial catalog escapes a failed pass.
pub fn recover_book(
    vault: &PrivateVault,
    anchors: &[BookAnchor],
) -> Result<BookCatalog, PeerError> {
    recover_with_scan(anchors, |names, visit| {
        vault.scan_optional_private_blobs(names, visit)
    })
}
pub(super) fn recover_scoped(
    scope: &mut nf_identity::private_storage::ProtectedBlobScope<'_>,
    anchors: &[BookAnchor],
) -> Result<BookCatalog, PeerError> {
    recover_with_scan(anchors, |names, visit| {
        scope.scan_optional_private_blobs(names, visit)
    })
}
fn recover_with_scan(
    anchors: &[BookAnchor],
    mut scan: impl FnMut(
        &[&str],
        &mut dyn for<'a> FnMut(usize, Option<&'a [u8]>) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError>,
) -> Result<BookCatalog, PeerError> {
    if anchors.len() > 8 {
        return Err(PeerError::Limit);
    }
    let mut configured: [Option<&BookAnchor>; 8] = [None; 8];
    for (index, anchor) in anchors.iter().enumerate() {
        if anchor.slot >= 8 || anchor.generation >= 8 || anchor.digest == [0; 32] {
            return Err(PeerError::Malformed);
        }
        if configured[anchor.slot as usize].is_some()
            || anchors[..index]
                .iter()
                .any(|old| old.original.request() == anchor.original.request())
        {
            return Err(PeerError::Malformed);
        }
        configured[anchor.slot as usize] = Some(anchor);
    }
    let names: [String; 64] =
        std::array::from_fn(|index| name((index / 8) as u8, (index % 8) as u8));
    let borrowed: [&str; 64] = std::array::from_fn(|index| names[index].as_str());
    let mut heads: [Option<BookRecord>; 8] = std::array::from_fn(|_| None);
    let mut anchored = [false; 8];
    let mut failure = None;
    let result = scan(&borrowed, &mut |index, payload| {
        let Some(payload) = payload else {
            return Ok(());
        };
        let slot = index / 8;
        let generation = (index % 8) as u8;
        let admitted = (|| {
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
        return Err(error);
    }
    result.map_err(|_| PeerError::Storage)?;
    for anchor in anchors {
        let slot = anchor.slot as usize;
        let Some(head) = &heads[slot] else {
            return Err(PeerError::Storage);
        };
        if !anchored[slot]
            || !head.minimum.admits(anchor.minimum)
            || head.minimum.membership_revision < anchor.original.source().minimum_membership
        {
            return Err(PeerError::Replay);
        }
    }
    Ok(BookCatalog { heads })
}
pub(super) fn name(slot: u8, generation: u8) -> String {
    format!("receipt-r{slot}-g{generation}")
}
