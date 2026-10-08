use super::TradeChallenge;
use crate::supplies::auth::{Context, Purpose};
use nf_kernel::trade::{
    accept_offer_digest, cancel_offer_digest, offer_status_digest, outbox_digest,
    reserve_offer_digest,
};
pub(super) fn context(request: TradeChallenge<'_>, membership: u64) -> Option<Context> {
    let (purpose, actor, device, digest) = match request {
        TradeChallenge::ReserveMaker(value) => (
            Purpose::TradeMaker,
            value.terms.maker,
            value.maker_device,
            reserve_offer_digest(value),
        ),
        TradeChallenge::ReserveTaker(value) => (
            Purpose::TradeTaker,
            value.terms.taker,
            value.taker_device,
            reserve_offer_digest(value),
        ),
        TradeChallenge::Accept(value) => (
            Purpose::TradeAccept,
            value.actor,
            value.device,
            accept_offer_digest(value),
        ),
        TradeChallenge::Cancel(value) => (
            Purpose::TradeCancel,
            value.actor,
            value.device,
            cancel_offer_digest(value),
        ),
        TradeChallenge::OfferStatus(value) => (
            Purpose::TradeOfferStatus,
            value.actor,
            value.device,
            offer_status_digest(value),
        ),
        TradeChallenge::Outbox(value) => (
            Purpose::TradeOutbox,
            value.actor,
            value.device,
            outbox_digest(value),
        ),
        _ => return None,
    };
    Some(Context {
        purpose,
        actor,
        device,
        membership,
        digest,
    })
}
