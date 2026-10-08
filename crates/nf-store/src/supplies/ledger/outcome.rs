use super::*;
use nf_kernel::supplies::{
    BurnOutcome, IssuanceOutcome, RejectedOutcome, RequestOutcome, RequestRejection,
    ReserveOutcome, StatusQuery,
};
use nf_kernel::trade::{
    CancelOffer, OfferId, OfferState, OfferStatusQuery, ReserveOffer, ReservedOffer,
    TradeFinalKind, TradeOutboxQuery, TradeReceipt, TradeRejection, TradeRequestOutcome,
    offer_terms_digest,
};

pub(crate) fn status(state: &State, query: &StatusQuery) -> Result<Option<TradeRequestOutcome>> {
    let Some((_, operation)) = state.requests.get(&query.request) else {
        return Ok(None);
    };
    let entry = state
        .operations
        .get(operation)
        .ok_or(SuppliesStoreError::Corrupt)?;
    let outcome = match entry.operation {
        JournalRecord::Supplies(value) => {
            if value.key().0 != query.owner {
                return Err(SuppliesRejection::Unauthorized.into());
            }
            let outcome = if entry.decision == Decision::InsufficientAvailable {
                RequestOutcome::Rejected(RejectedOutcome {
                    operation: value.id(),
                    revision: entry.revision,
                    reason: RequestRejection::InsufficientAvailable,
                })
            } else {
                match value {
                    Operation::Issue(v) => RequestOutcome::Issued(IssuanceOutcome {
                        issuance: v.issuance,
                        revision: entry.revision,
                    }),
                    Operation::Burn(v) => RequestOutcome::Burned(BurnOutcome {
                        burn: v.burn,
                        revision: entry.revision,
                    }),
                    Operation::Reserve(v) => RequestOutcome::Reserved(ReserveOutcome {
                        reservation: v.reservation,
                        revision: entry.revision,
                    }),
                }
            };
            TradeRequestOutcome::Supplies(outcome)
        }
        JournalRecord::ReserveOffer(value) => {
            party(value, query.actor)?;
            trade_decision(*entry)?
        }
        JournalRecord::AcceptOffer(value) => {
            let (history, original, _) = original_offer(state, value.offer)?;
            party(original, query.actor)?;
            if history.finalization != Some(value.operation) {
                return Err(SuppliesStoreError::Corrupt);
            }
            TradeRequestOutcome::Accepted(accept_receipt(*entry)?)
        }
        JournalRecord::CancelOffer(value) => {
            let (history, original, _) = original_offer(state, value.offer)?;
            party(original, query.actor)?;
            if history.finalization != Some(value.operation) {
                return Err(SuppliesStoreError::Corrupt);
            }
            TradeRequestOutcome::Cancelled(cancel_receipt(*entry)?)
        }
    };
    Ok(Some(outcome))
}
fn party(value: nf_kernel::trade::ReserveOffer, actor: AccountId) -> Result<()> {
    if actor != value.terms.maker && actor != value.terms.taker {
        return Err(SuppliesRejection::Unauthorized.into());
    }
    Ok(())
}
pub(super) fn trade_decision(entry: Entry) -> Result<TradeRequestOutcome> {
    let JournalRecord::ReserveOffer(value) = entry.operation else {
        return Err(SuppliesStoreError::Corrupt);
    };
    Ok(match entry.decision {
        Decision::Accepted => TradeRequestOutcome::Reserved {
            operation: value.operation,
            offer: ReservedOffer {
                offer: value.terms.offer,
                version: value.terms.version,
                revision: entry.revision,
                digest: offer_terms_digest(&value.terms),
            },
        },
        Decision::InsufficientAvailable => TradeRequestOutcome::Rejected {
            operation: value.operation,
            revision: entry.revision,
            reason: TradeRejection::InsufficientAvailable,
        },
    })
}
fn original_offer(state: &State, offer: OfferId) -> Result<(OfferHistory, ReserveOffer, Entry)> {
    let history = *state
        .offers
        .get(&offer)
        .ok_or(SuppliesRejection::Conflict)?;
    let entry = *state
        .operations
        .get(&history.reservation)
        .ok_or(SuppliesStoreError::Corrupt)?;
    let JournalRecord::ReserveOffer(value) = entry.operation else {
        return Err(SuppliesStoreError::Corrupt);
    };
    if entry.decision != Decision::Accepted
        || value.terms.offer != offer
        || value.operation != history.reservation
    {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok((history, value, entry))
}
/// Pure original-target resolution for a novel acceptance after global dedupe.
pub(super) fn accept_target(
    state: &State,
    value: &nf_kernel::trade::AcceptOffer,
) -> Result<ReserveOffer> {
    let (history, original, _) = original_offer(state, value.offer)?;
    if original.terms.taker != value.actor {
        return Err(SuppliesRejection::Unauthorized.into());
    }
    if history.finalization.is_some()
        || original.terms.version != value.version
        || offer_terms_digest(&original.terms) != value.digest
    {
        return Err(SuppliesRejection::Conflict.into());
    }
    Ok(original)
}
/// Pure target lookup, performed only for a novel operation after global dedupe.
pub(super) fn cancel_target(state: &State, cancel: &CancelOffer) -> Result<ReserveOffer> {
    let (history, original, _) = original_offer(state, cancel.offer)?;
    party(original, cancel.actor)?;
    if history.finalization.is_some()
        || original.terms.version != cancel.version
        || offer_terms_digest(&original.terms) != cancel.digest
    {
        return Err(SuppliesRejection::Conflict.into());
    }
    Ok(original)
}
pub(super) fn cancel_receipt(entry: Entry) -> Result<TradeReceipt> {
    let JournalRecord::CancelOffer(value) = entry.operation else {
        return Err(SuppliesStoreError::Corrupt);
    };
    if entry.decision != Decision::Accepted {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(TradeReceipt {
        kind: TradeFinalKind::Cancelled,
        operation: value.operation,
        revision: entry.revision,
        offer: value.offer,
        version: value.version,
        digest: value.digest,
    })
}
pub(super) fn accept_receipt(entry: Entry) -> Result<TradeReceipt> {
    let JournalRecord::AcceptOffer(value) = entry.operation else {
        return Err(SuppliesStoreError::Corrupt);
    };
    if entry.decision != Decision::Accepted {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(TradeReceipt {
        kind: TradeFinalKind::Accepted,
        operation: value.operation,
        revision: entry.revision,
        offer: value.offer,
        version: value.version,
        digest: value.digest,
    })
}
fn final_receipt(entry: Entry) -> Result<TradeReceipt> {
    match entry.operation {
        JournalRecord::CancelOffer(_) => cancel_receipt(entry),
        JournalRecord::AcceptOffer(_) => accept_receipt(entry),
        _ => Err(SuppliesStoreError::Corrupt),
    }
}
pub(crate) fn offer_state(state: &State, query: &OfferStatusQuery) -> Result<Option<OfferState>> {
    if state.offers.contains_key(&query.offer) {
        let (history, original, entry) = original_offer(state, query.offer)?;
        party(original, query.actor)?;
        if original.terms.version != query.version {
            return Ok(None);
        }
        if let Some(operation) = history.finalization {
            let final_entry = *state
                .operations
                .get(&operation)
                .ok_or(SuppliesStoreError::Corrupt)?;
            let receipt = final_receipt(final_entry)?;
            if receipt.offer != original.terms.offer
                || receipt.version != original.terms.version
                || receipt.digest != offer_terms_digest(&original.terms)
            {
                return Err(SuppliesStoreError::Corrupt);
            }
            return Ok(Some(OfferState::Closed(receipt)));
        }
        return match trade_decision(entry)? {
            TradeRequestOutcome::Reserved { offer, .. } => Ok(Some(OfferState::Reserved(offer))),
            _ => Err(SuppliesStoreError::Corrupt),
        };
    }
    for entry in state.operations.values() {
        if let JournalRecord::ReserveOffer(value) = entry.operation
            && value.terms.offer == query.offer
            && value.terms.version == query.version
        {
            party(value, query.actor)?;
            if entry.decision != Decision::InsufficientAvailable {
                return Err(SuppliesStoreError::Corrupt);
            }
        }
    }
    Ok(None)
}
pub(crate) fn offer_status(
    state: &State,
    query: &OfferStatusQuery,
) -> Result<Option<ReservedOffer>> {
    Ok(match offer_state(state, query)? {
        Some(OfferState::Reserved(offer)) => Some(offer),
        Some(OfferState::Closed(_)) | None => None,
    })
}
/// One logical receipt per durable final operation, visible to either original party.
pub(crate) fn outbox(state: &State, query: &TradeOutboxQuery) -> Result<Vec<TradeReceipt>> {
    let mut receipts = Vec::new();
    for entry in state.operations.values() {
        if matches!(
            entry.operation,
            JournalRecord::CancelOffer(_) | JournalRecord::AcceptOffer(_)
        ) {
            let receipt = final_receipt(*entry)?;
            let (history, original, _) = original_offer(state, receipt.offer)?;
            if history.finalization != Some(receipt.operation) {
                return Err(SuppliesStoreError::Corrupt);
            }
            if (query.actor == original.terms.maker || query.actor == original.terms.taker)
                && entry.revision > query.after_revision
            {
                receipts.push(receipt);
            }
        }
    }
    receipts.sort_unstable_by_key(|receipt| receipt.revision);
    receipts.truncate(query.limit as usize);
    Ok(receipts)
}
