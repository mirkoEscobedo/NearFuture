use super::supplies_support::{ChallengeSource, Fixture};
use nf_contract::identity::{AccountId, OperationId, RequestId};
use nf_kernel::{
    supplies::{BalanceQuery, IssuanceReason, IssuerRule, Origin, TrustClass, policy_digest},
    trade::{
        AcceptRule, AcceptancePolicy, AssetTerms, CancelRule, OfferId, OfferTerms, ReserveOffer,
        TradeAdmission, TradeBalance, TradePolicy,
    },
};
use nf_store::{
    supplies::ProofAttempt,
    trade::{TradeChallenge, TradeStore},
};
impl<'a> ChallengeSource<TradeChallenge<'a>> for TradeStore {
    fn issued(&mut self, value: TradeChallenge<'a>) -> nf_store::supplies::IssuedChallenge {
        self.issue_challenge(value).unwrap()
    }
}
pub(super) struct AcceptFixture {
    pub(super) supplies: Fixture,
    pub(super) policy: AcceptancePolicy,
    pub(super) g: Origin,
    pub(super) w: Origin,
}
impl AcceptFixture {
    pub(super) fn new() -> Self {
        let mut supplies = Fixture::new();
        let g = supplies.policy.issuers[0].origin;
        let w = Origin {
            trust: TrustClass::Canonical,
            lineage: [9; 32],
        };
        // A second real canonical issuer rule is selected BEFORE repository creation.
        supplies.policy.issuers.push(IssuerRule {
            issuer: supplies.issuer.account,
            content: nf_kernel::supplies::SUPPLIES_CONTENT,
            origin: w,
            reason: IssuanceReason::AuthorityGrant,
            maximum: 1000,
        });
        let trade = TradePolicy {
            supplies: supplies.policy.clone(),
            clock_authority: supplies.issuer.account,
            allowed_origins: vec![g, w],
            admission: TradeAdmission::AuthorityVaultOnly,
            cancel_rule: CancelRule::EitherNamedParty,
        };
        let policy = AcceptancePolicy {
            trade,
            accept_rule: AcceptRule::NamedTaker,
        };
        Self {
            supplies,
            policy,
            g,
            w,
        }
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
        value: TradeChallenge<'_>,
    ) -> ProofAttempt {
        self.supplies.attempt(store, value)
    }
    pub(super) fn grant(
        &self,
        store: &mut TradeStore,
        owner: AccountId,
        origin: Origin,
        amount: u64,
        ids: (u8, u8),
        revision: u64,
    ) {
        let mut value = self.supplies.issuance();
        value.request = RequestId::from_bytes([ids.0; 16]);
        value.issuance = OperationId::from_bytes([ids.1; 16]);
        value.policy = policy_digest(&self.policy.trade.supplies).unwrap();
        value.beneficiary = owner;
        value.origin = origin;
        value.amount = amount;
        let proof = self.attempt(store, TradeChallenge::Issue(&value));
        assert_eq!(
            store.issue(&value, proof).unwrap().unwrap().revision,
            revision
        );
    }
    pub(super) fn reserve(&self) -> ReserveOffer {
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
                universe: self.policy.trade.supplies.universe,
                history: self.policy.trade.supplies.history,
                policy: self.policy.digest().unwrap(),
                give: AssetTerms {
                    content: nf_kernel::supplies::SUPPLIES_CONTENT,
                    origin: self.g,
                    amount: 8,
                },
                want: AssetTerms {
                    content: nf_kernel::supplies::SUPPLIES_CONTENT,
                    origin: self.w,
                    amount: 5,
                },
                expires_at: 10,
            },
        }
    }
    pub(super) fn query(&self, owner: AccountId, origin: Origin) -> BalanceQuery {
        let mut value = self.supplies.query();
        value.actor = owner;
        value.owner = owner;
        value.origin = origin;
        value.device = if owner == self.maker() {
            self.supplies.beneficiary.device
        } else {
            assert_eq!(owner, self.taker());
            self.supplies.issuer.device
        };
        value
    }
    pub(super) fn balance(
        &self,
        store: &mut TradeStore,
        owner: AccountId,
        origin: Origin,
    ) -> TradeBalance {
        let query = self.query(owner, origin);
        let proof = self.attempt(store, TradeChallenge::Balance(&query));
        store.trade_balance(&query, proof).unwrap()
    }
}
