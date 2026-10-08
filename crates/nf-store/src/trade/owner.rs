use super::{
    KnownTradeFrontiers, TradeChallenge, auth,
    error::{Result, TradeStoreError},
    query, schema,
};
use crate::supplies::{
    IssuedChallenge, ProofAttempt, SuppliesStore, SuppliesStoreError, auth as shared_auth, ledger,
    membership,
};
use nf_identity::model::MembershipState;
use nf_kernel::{
    supplies::{BalanceQuery, Balances, Burn, BurnOutcome, Issuance, IssuanceOutcome, StatusQuery},
    trade::{
        AcceptOffer, AcceptancePolicy, CancelOffer, OfferState, OfferStatusQuery, ReserveOffer,
        ReservedOffer, TradeBalance, TradeOutboxQuery, TradePolicy, TradeReceipt, TradeRejection,
        TradeRequestOutcome,
    },
};
use rusqlite::TransactionBehavior;
use std::path::Path;
pub struct TradeStore {
    inner: SuppliesStore,
    policy: super::policy::SelectedPolicy,
}
impl TradeStore {
    pub fn create(
        path: impl AsRef<Path>,
        policy: &TradePolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        Self::create_selected(
            path,
            super::policy::SelectedPolicy::Legacy2(policy.clone()),
            membership,
        )
    }
    pub fn create_accepting(
        path: impl AsRef<Path>,
        policy: &AcceptancePolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        Self::create_selected(
            path,
            super::policy::SelectedPolicy::Accepting3(policy.clone()),
            membership,
        )
    }
    fn create_selected(
        path: impl AsRef<Path>,
        policy: super::policy::SelectedPolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        policy.validate()?;
        if policy.base().clock_authority != membership.owner {
            return Err(TradeRejection::Policy.into());
        }
        let inner = SuppliesStore::create_profile(
            path,
            policy.supplies(),
            membership,
            Some(policy.clone()),
        )?;
        Ok(Self { inner, policy })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        policy: &TradePolicy,
        known: KnownTradeFrontiers,
    ) -> Result<Self> {
        Self::open_selected(
            path,
            super::policy::SelectedPolicy::Legacy2(policy.clone()),
            known,
        )
    }
    pub fn open_accepting(
        path: impl AsRef<Path>,
        policy: &AcceptancePolicy,
        known: KnownTradeFrontiers,
    ) -> Result<Self> {
        Self::open_selected(
            path,
            super::policy::SelectedPolicy::Accepting3(policy.clone()),
            known,
        )
    }
    fn open_selected(
        path: impl AsRef<Path>,
        policy: super::policy::SelectedPolicy,
        known: KnownTradeFrontiers,
    ) -> Result<Self> {
        policy.validate()?;
        if known.clock_tick != 0 || known.clock_revision != 0 {
            return Err(SuppliesStoreError::StaleBackup.into());
        }
        let inner = SuppliesStore::open_profile(
            path,
            policy.supplies(),
            known.supplies(),
            Some(policy.clone()),
        )?;
        Ok(Self { inner, policy })
    }
    pub fn known_frontiers(&self) -> Result<KnownTradeFrontiers> {
        self.inner.current()?;
        let tx = self.inner.connection.unchecked_transaction()?;
        let member = membership::load(&tx, self.inner.scope())?;
        let state = ledger::verify(&tx, self.policy.supplies(), Some(&self.policy))?;
        let (clock_tick, clock_revision) = schema::verify(&tx, &self.policy)?;
        let known = KnownTradeFrontiers {
            scope: member.scope,
            revision: state.revision,
            membership_revision: member.revision,
            clock_tick,
            clock_revision,
        };
        tx.commit()?;
        Ok(known)
    }
    pub fn issue_challenge(&mut self, request: TradeChallenge<'_>) -> Result<IssuedChallenge> {
        if let Some(request) = request.supplies() {
            return Ok(self.inner.issue_challenge(request)?);
        }
        let member = self.inner.current()?;
        let context =
            auth::context(request, member.revision).ok_or(TradeStoreError::UnsupportedOperation)?;
        Ok(self
            .inner
            .auth
            .issue_context(context, &member, self.policy.digest()?)?)
    }
    pub fn issue(
        &mut self,
        issue: &Issuance,
        proof: ProofAttempt,
    ) -> Result<Option<IssuanceOutcome>> {
        Ok(self.inner.issue(issue, proof)?)
    }
    /// Delegates authority burns to the existing authenticated supplies transaction.
    pub fn burn(&mut self, burn: &Burn, proof: ProofAttempt) -> Result<Option<BurnOutcome>> {
        Ok(self.inner.burn(burn, proof)?)
    }
    pub fn balance(&mut self, query: &BalanceQuery, proof: ProofAttempt) -> Result<Balances> {
        Ok(self.inner.balance(query, proof)?)
    }
    pub fn status(
        &mut self,
        query: &StatusQuery,
        proof: ProofAttempt,
    ) -> Result<Option<TradeRequestOutcome>> {
        Ok(self.inner.status_mixed(query, proof)?)
    }
    /// A jointly authenticated decision reserves both sides or durably refuses without effects.
    pub fn reserve_offer(
        &mut self,
        value: &ReserveOffer,
        maker: ProofAttempt,
        taker: ProofAttempt,
    ) -> Result<ReservedOffer> {
        let maker = self.inner.auth.take(maker)?;
        let taker = self.inner.auth.take(taker)?;
        if self.inner.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.inner.scope();
        let tx = self
            .inner
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let member = membership::load(&tx, scope)?;
        shared_auth::verify_context(
            &maker,
            auth::context(TradeChallenge::ReserveMaker(value), member.revision)
                .ok_or(TradeStoreError::UnsupportedOperation)?,
            &member,
        )?;
        shared_auth::verify_context(
            &taker,
            auth::context(TradeChallenge::ReserveTaker(value), member.revision)
                .ok_or(TradeStoreError::UnsupportedOperation)?,
            &member,
        )?;
        self.policy.permit_offer(&value.terms)?;
        let state = ledger::verify(&tx, self.policy.supplies(), Some(&self.policy))?;
        shared_auth::live(&maker)?;
        shared_auth::live(&taker)?;
        let decision = ledger::reserve_offer(&tx, *value, &state).map_err(|error| match error {
            SuppliesStoreError::Rejected(nf_kernel::supplies::SuppliesRejection::Conflict) => {
                TradeStoreError::Rejected(TradeRejection::Conflict)
            }
            other => TradeStoreError::Supplies(other),
        })?;

        shared_auth::live(&maker)?;
        shared_auth::live(&taker)?;
        if let Err(error) = tx.commit() {
            self.inner.quarantined = true;
            return Err(error.into());
        }
        shared_auth::live(&maker)?;
        shared_auth::live(&taker)?;
        match decision {
            TradeRequestOutcome::Reserved { offer, .. } => Ok(offer),
            TradeRequestOutcome::Rejected { reason, .. } => Err(reason.into()),
            TradeRequestOutcome::Supplies(_)
            | TradeRequestOutcome::Cancelled(_)
            | TradeRequestOutcome::Accepted(_) => Err(SuppliesStoreError::Corrupt.into()),
        }
    }
    /// One authenticated authority transaction releases the original joint escrow once.
    pub fn cancel_offer(
        &mut self,
        value: &CancelOffer,
        proof: ProofAttempt,
    ) -> Result<TradeReceipt> {
        let evidence = self.inner.auth.take(proof)?;
        if self.inner.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.inner.scope();
        let tx = self
            .inner
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let member = membership::load(&tx, scope)?;
        shared_auth::verify_context(
            &evidence,
            auth::context(TradeChallenge::Cancel(value), member.revision)
                .ok_or(TradeStoreError::UnsupportedOperation)?,
            &member,
        )?;
        self.policy.permit_cancel(value)?;
        let state = ledger::verify(&tx, self.policy.supplies(), Some(&self.policy))?;
        shared_auth::live(&evidence)?;
        let receipt = ledger::cancel_offer(&tx, *value, &state).map_err(|error| match error {
            SuppliesStoreError::Rejected(nf_kernel::supplies::SuppliesRejection::Conflict) => {
                TradeStoreError::Rejected(TradeRejection::Conflict)
            }
            SuppliesStoreError::Rejected(nf_kernel::supplies::SuppliesRejection::Unauthorized) => {
                TradeStoreError::Rejected(TradeRejection::Unauthorized)
            }
            other => TradeStoreError::Supplies(other),
        })?;
        shared_auth::live(&evidence)?;
        if let Err(error) = tx.commit() {
            self.inner.quarantined = true;
            return Err(error.into());
        }
        shared_auth::live(&evidence)?;
        Ok(receipt)
    }
    /// One current named-taker authority transaction exchanges the original joint escrow once.
    pub fn accept_offer(
        &mut self,
        value: &AcceptOffer,
        proof: ProofAttempt,
    ) -> Result<TradeReceipt> {
        let evidence = self.inner.auth.take(proof)?;
        if self.inner.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.inner.scope();
        let tx = self
            .inner
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let member = membership::load(&tx, scope)?;
        shared_auth::verify_context(
            &evidence,
            auth::context(TradeChallenge::Accept(value), member.revision)
                .ok_or(TradeStoreError::UnsupportedOperation)?,
            &member,
        )?;
        let policy = self
            .policy
            .accepting()
            .ok_or(TradeStoreError::UnsupportedOperation)?;
        policy.permit_accept(value)?;
        let state = ledger::verify(&tx, self.policy.supplies(), Some(&self.policy))?;
        shared_auth::live(&evidence)?;
        let receipt = ledger::accept_offer(&tx, *value, &state).map_err(|error| match error {
            SuppliesStoreError::Rejected(nf_kernel::supplies::SuppliesRejection::Conflict) => {
                TradeStoreError::Rejected(TradeRejection::Conflict)
            }
            SuppliesStoreError::Rejected(nf_kernel::supplies::SuppliesRejection::Unauthorized) => {
                TradeStoreError::Rejected(TradeRejection::Unauthorized)
            }
            other => TradeStoreError::Supplies(other),
        })?;
        shared_auth::live(&evidence)?;
        if let Err(error) = tx.commit() {
            self.inner.quarantined = true;
            return Err(error.into());
        }
        shared_auth::live(&evidence)?;
        Ok(receipt)
    }
    /// Stock and transfer counters are projected from the SAME authenticated transaction.
    pub fn trade_balance(
        &mut self,
        query: &BalanceQuery,
        proof: ProofAttempt,
    ) -> Result<TradeBalance> {
        Ok(self.inner.trade_balance(query, proof)?)
    }
    pub fn offer_status(
        &mut self,
        value: &OfferStatusQuery,
        proof: ProofAttempt,
    ) -> Result<Option<ReservedOffer>> {
        query::read(
            &mut self.inner,
            &self.policy,
            TradeChallenge::OfferStatus(value),
            proof,
            value.universe,
            value.history,
            |state| ledger::offer_status(state, value),
        )
    }
    /// Both offer projections share the exact signed query and Economic read purpose.
    /// A separate fresh one-use proof is required for each projection call.
    pub fn offer_state(
        &mut self,
        value: &OfferStatusQuery,
        proof: ProofAttempt,
    ) -> Result<Option<OfferState>> {
        query::read(
            &mut self.inner,
            &self.policy,
            TradeChallenge::OfferStatus(value),
            proof,
            value.universe,
            value.history,
            |state| ledger::offer_state(state, value),
        )
    }
    pub fn outbox(
        &mut self,
        value: &TradeOutboxQuery,
        proof: ProofAttempt,
    ) -> Result<Vec<TradeReceipt>> {
        query::read(
            &mut self.inner,
            &self.policy,
            TradeChallenge::Outbox(value),
            proof,
            value.universe,
            value.history,
            |state| ledger::outbox(state, value),
        )
    }
}

/// Trusted persistence port; signed admission is enforced by the identity entry points.
impl nf_identity::model::MembershipRepository for TradeStore {
    fn load_membership(
        &mut self,
        scope: nf_identity::model::Scope,
    ) -> std::result::Result<Option<MembershipState>, nf_identity::model::IdentityError> {
        nf_identity::model::MembershipRepository::load_membership(&mut self.inner, scope)
    }
    fn commit_membership(
        &mut self,
        expected_revision: Option<u64>,
        next: &MembershipState,
    ) -> std::result::Result<(), nf_identity::model::IdentityError> {
        nf_identity::model::MembershipRepository::commit_membership(
            &mut self.inner,
            expected_revision,
            next,
        )
    }
}
