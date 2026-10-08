use crate::trade_support::TradeFixture;
use nf_kernel::trade::{OfferStatusQuery, ReserveOffer, ReservedOffer, TradeOutboxQuery};
use nf_store::trade::{TradeChallenge, TradeStore, TradeStoreError as Error};

impl TradeFixture {
    pub(super) fn assert_no_open_offer_or_receipt(
        &self,
        store: &mut TradeStore,
        offer: &ReserveOffer,
    ) {
        let query = OfferStatusQuery {
            actor: self.maker(),
            device: self.supplies.beneficiary.device,
            universe: offer.terms.universe,
            history: offer.terms.history,
            offer: offer.terms.offer,
            version: offer.terms.version,
        };
        let proof = self.attempt(store, TradeChallenge::OfferStatus(&query));
        assert_eq!(store.offer_status(&query, proof).unwrap(), None);
        let query = TradeOutboxQuery {
            actor: self.maker(),
            device: self.supplies.beneficiary.device,
            universe: offer.terms.universe,
            history: offer.terms.history,
            after_revision: 0,
            limit: 16,
        };
        let proof = self.attempt(store, TradeChallenge::Outbox(&query));
        assert!(store.outbox(&query, proof).unwrap().is_empty());
    }
}

impl TradeFixture {
    pub(super) fn funded(&self, scratch: &super::supplies_support::Scratch) -> TradeStore {
        let mut store =
            TradeStore::create(scratch.db(), &self.policy, &self.supplies.membership).unwrap();
        self.grant(&mut store, self.maker(), 25, 80, 81, 1);
        self.grant(&mut store, self.taker(), 4, 82, 83, 2);
        store
    }
    pub(super) fn refuse(
        &self,
        store: &mut TradeStore,
        value: &ReserveOffer,
    ) -> Result<ReservedOffer, Error> {
        let maker = self.attempt(store, TradeChallenge::ReserveMaker(value));
        let taker = self.attempt(store, TradeChallenge::ReserveTaker(value));
        store.reserve_offer(value, maker, taker)
    }
}
