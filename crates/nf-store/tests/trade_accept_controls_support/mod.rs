use super::{supplies_support::Scratch, trade_accept_support::AcceptFixture};
use nf_contract::identity::{AccountId, OperationId, RequestId};
use nf_kernel::{
    supplies::{Balances, StatusQuery},
    trade::{
        AcceptOffer, CancelOffer, OfferState, OfferStatusQuery, ReserveOffer, ReservedOffer,
        TradeBalance, TradeFinalKind, TradeOutboxQuery, TradeReceipt, TradeRequestOutcome,
    },
};
use nf_store::{
    supplies::ProofAttempt,
    trade::{TradeChallenge, TradeStore, TradeStoreError as Error},
};
use std::path::Path;

pub(super) fn snapshot(stock: [u64; 6], received: u64, sent: u64) -> TradeBalance {
    TradeBalance {
        stock: Balances {
            available: stock[0],
            reserved: stock[1],
            pending: stock[2],
            externalized: stock[3],
            minted: stock[4],
            burned: stock[5],
        },
        received,
        sent,
    }
}
pub(super) fn files(db: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(db.parent().unwrap())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}
pub(super) fn copy_proof(value: &ProofAttempt) -> ProofAttempt {
    ProofAttempt {
        ticket: value.ticket,
        proof: value.proof.clone(),
    }
}
pub(super) struct Scenario {
    pub store: TradeStore,
    pub f: AcceptFixture,
    pub scratch: Scratch,
    pub reserve: ReserveOffer,
    pub reserved: ReservedOffer,
    pub accept: AcceptOffer,
}
impl Scenario {
    pub fn new(same_origin: bool) -> Self {
        let scratch = Scratch::new();
        let f = AcceptFixture::new();
        let mut store =
            TradeStore::create_accepting(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
        f.grant(&mut store, f.maker(), f.g, 25, (80, 81), 1);
        f.grant(
            &mut store,
            f.taker(),
            if same_origin { f.g } else { f.w },
            10,
            (82, 83),
            2,
        );
        let mut reserve = f.reserve();
        if same_origin {
            reserve.terms.want.origin = f.g;
        }
        // Both actual consents bind the final original terms, including the chosen origin.
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
        let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
        assert_eq!(reserved.revision, 3);
        assert_eq!(store.known_frontiers().unwrap().revision, 3);
        let accept = AcceptOffer {
            request: RequestId::from_bytes([92; 16]),
            operation: OperationId::from_bytes([91; 16]),
            actor: f.taker(),
            device: f.supplies.issuer.device,
            universe: reserve.terms.universe,
            history: reserve.terms.history,
            policy: f.policy.digest().unwrap(),
            offer: reserved.offer,
            version: 1,
            digest: reserved.digest,
        };
        Self {
            scratch,
            f,
            store,
            reserve,
            reserved,
            accept,
        }
    }
    pub fn accepted(&self) -> TradeReceipt {
        TradeReceipt {
            kind: TradeFinalKind::Accepted,
            operation: OperationId::from_bytes([91; 16]),
            revision: 4,
            offer: self.reserved.offer,
            version: 1,
            digest: self.reserved.digest,
        }
    }
    pub fn cancel(&self, reserved: ReservedOffer, actor: AccountId, ids: (u8, u8)) -> CancelOffer {
        let query = self.f.query(actor, self.f.g);
        CancelOffer {
            request: RequestId::from_bytes([ids.0; 16]),
            operation: OperationId::from_bytes([ids.1; 16]),
            actor,
            device: query.device,
            universe: query.universe,
            history: query.history,
            policy: self.f.policy.digest().unwrap(),
            offer: reserved.offer,
            version: reserved.version,
            digest: reserved.digest,
        }
    }
    pub fn status(&mut self, actor: AccountId, request: RequestId) -> Option<TradeRequestOutcome> {
        let query = self.f.query(actor, self.f.g);
        let value = StatusQuery {
            actor,
            device: query.device,
            owner: actor,
            universe: query.universe,
            history: query.history,
            request,
        };
        let proof = self
            .f
            .attempt(&mut self.store, TradeChallenge::Status(&value));
        self.store.status(&value, proof).unwrap()
    }
    pub fn closed_and_outbox(&mut self, receipt: TradeReceipt, receipts: &[TradeReceipt]) {
        for actor in [self.f.maker(), self.f.taker()] {
            let own = self.f.query(actor, self.f.g);
            let query = OfferStatusQuery {
                actor,
                device: own.device,
                universe: own.universe,
                history: own.history,
                offer: receipt.offer,
                version: receipt.version,
            };
            let proof = self
                .f
                .attempt(&mut self.store, TradeChallenge::OfferStatus(&query));
            assert_eq!(
                self.store.offer_state(&query, proof).unwrap(),
                Some(OfferState::Closed(receipt))
            );
            let proof = self
                .f
                .attempt(&mut self.store, TradeChallenge::OfferStatus(&query));
            assert_eq!(self.store.offer_status(&query, proof).unwrap(), None);
            let query = TradeOutboxQuery {
                actor,
                device: own.device,
                universe: own.universe,
                history: own.history,
                after_revision: 0,
                limit: 64,
            };
            let proof = self
                .f
                .attempt(&mut self.store, TradeChallenge::Outbox(&query));
            assert_eq!(self.store.outbox(&query, proof).unwrap(), receipts);
            assert_eq!(
                self.status(actor, self.reserve.request),
                Some(TradeRequestOutcome::Reserved {
                    operation: OperationId::from_bytes([85; 16]),
                    offer: self.reserved
                })
            );
        }
    }
    pub fn refuse(
        &mut self,
        value: AcceptOffer,
        expected: Error,
        retained: Option<TradeRequestOutcome>,
    ) {
        let known = self.store.known_frontiers().unwrap();
        let proof = self
            .f
            .attempt(&mut self.store, TradeChallenge::Accept(&value));
        // The valid owner is already configured; only this actual refused call is byte-compared.
        let before = files(&self.scratch.db());
        assert_eq!(self.store.accept_offer(&value, proof), Err(expected));
        assert_eq!(files(&self.scratch.db()), before);
        assert_eq!(self.store.known_frontiers().unwrap(), known);
        assert_eq!(self.status(self.f.taker(), value.request), retained);
        assert_eq!(self.store.known_frontiers().unwrap(), known);
    }
}
