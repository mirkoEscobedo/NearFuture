use super::supplies_support::{ChallengeSource, Fixture};
use nf_contract::identity::{AccountId, OperationId, RequestId};
use nf_kernel::supplies::{BalanceQuery, IssuanceOutcome, StatusQuery};
use nf_kernel::trade::{
    AssetTerms, CancelRule, OfferId, OfferTerms, ReserveOffer, TradeAdmission, TradePolicy,
    TradeRequestOutcome,
};
use nf_store::supplies::ProofAttempt;
use nf_store::trade::{TradeChallenge, TradeStore};

impl<'a> ChallengeSource<TradeChallenge<'a>> for TradeStore {
    fn issued(&mut self, request: TradeChallenge<'a>) -> nf_store::supplies::IssuedChallenge {
        self.issue_challenge(request).unwrap()
    }
}

pub(super) struct TradeFixture {
    pub(super) supplies: Fixture,
    pub(super) policy: TradePolicy,
}
impl TradeFixture {
    pub(super) fn new() -> Self {
        let supplies = Fixture::new();
        let policy = TradePolicy {
            supplies: supplies.policy.clone(),
            clock_authority: supplies.issuer.account,
            allowed_origins: vec![supplies.policy.issuers[0].origin],
            admission: TradeAdmission::AuthorityVaultOnly,
            cancel_rule: CancelRule::EitherNamedParty,
        };
        Self { supplies, policy }
    }
    pub(super) fn maker(&self) -> AccountId {
        self.supplies.beneficiary.account
    }
    pub(super) fn taker(&self) -> AccountId {
        self.supplies.issuer.account
    }
    pub(super) fn attempt(
        &self,
        store: &mut TradeStore,
        request: TradeChallenge<'_>,
    ) -> ProofAttempt {
        self.supplies.attempt(store, request)
    }
    pub(super) fn grant(
        &self,
        store: &mut TradeStore,
        owner: AccountId,
        amount: u64,
        request: u8,
        operation: u8,
        revision: u64,
    ) {
        let mut issue = self.supplies.issuance();
        issue.request = RequestId::from_bytes([request; 16]);
        issue.issuance = OperationId::from_bytes([operation; 16]);
        issue.beneficiary = owner;
        issue.amount = amount;
        let proof = self.attempt(store, TradeChallenge::Issue(&issue));
        assert_eq!(
            store.issue(&issue, proof).unwrap(),
            Some(IssuanceOutcome {
                issuance: issue.issuance,
                revision,
            })
        );
    }
    pub(super) fn balance(&self, store: &mut TradeStore, owner: AccountId) -> [u64; 6] {
        let mut query: BalanceQuery = self.supplies.query();
        query.owner = owner;
        query.actor = owner;
        query.device = if owner == self.maker() {
            self.supplies.beneficiary.device
        } else {
            assert_eq!(owner, self.taker());
            self.supplies.issuer.device
        };
        let proof = self.attempt(store, TradeChallenge::Balance(&query));
        let balance = store.balance(&query, proof).unwrap();
        [
            balance.available,
            balance.reserved,
            balance.pending,
            balance.externalized,
            balance.minted,
            balance.burned,
        ]
    }
    pub(super) fn reserve_offer(&self) -> ReserveOffer {
        let query = self.supplies.query();
        let asset = |amount| AssetTerms {
            content: query.content,
            origin: query.origin,
            amount,
        };
        ReserveOffer {
            request: RequestId::from_bytes([84; 16]),
            operation: OperationId::from_bytes([85; 16]),
            maker_device: self.supplies.beneficiary.device,
            taker_device: self.supplies.issuer.device,
            terms: OfferTerms {
                offer: OfferId::from_bytes([90; 16]),
                version: 1,
                maker: self.maker(),
                taker: self.taker(),
                universe: query.universe,
                history: query.history,
                policy: self.policy.digest().unwrap(),
                give: asset(8),
                want: asset(5),
                expires_at: 10,
            },
        }
    }
    pub(super) fn request_status(
        &self,
        store: &mut TradeStore,
        owner: AccountId,
        request: RequestId,
    ) -> Option<TradeRequestOutcome> {
        let stock = self.supplies.query();
        let query = StatusQuery {
            actor: owner,
            owner,
            device: if owner == self.maker() {
                self.supplies.beneficiary.device
            } else {
                assert_eq!(owner, self.taker());
                self.supplies.issuer.device
            },
            universe: stock.universe,
            history: stock.history,
            request,
        };
        let proof = self.attempt(store, TradeChallenge::Status(&query));
        store.status(&query, proof).unwrap()
    }
}
