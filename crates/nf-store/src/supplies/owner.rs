use super::{
    ChallengeRequest, IssuedChallenge, KnownSuppliesFrontiers, ProofAttempt,
    auth::{self, AuthRuntime},
    error::{Result, SuppliesStoreError},
    ledger, membership, schema,
};
use nf_identity::model::{MembershipState, Scope};
use nf_kernel::supplies::{
    BalanceQuery, Balances, Burn, BurnOutcome, Issuance, IssuanceOutcome, RejectedOutcome,
    RequestOutcome, RequestRejection, Reserve, ReserveOutcome, SUPPLIES_CONTENT, StatusQuery,
    SuppliesPolicy, SuppliesRejection, policy_digest,
};
use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;
pub struct SuppliesStore {
    pub(super) connection: Connection,
    policy: SuppliesPolicy,
    auth: AuthRuntime,
    pub(super) quarantined: bool,
}
impl SuppliesStore {
    pub fn create(
        path: impl AsRef<Path>,
        policy: &SuppliesPolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        policy.validate()?;
        let scope = Scope {
            universe: policy.universe,
            history: policy.history,
        };
        if membership.scope != scope {
            return Err(SuppliesRejection::Scope.into());
        }
        nf_identity::codec::validate_state(membership)?;
        for rule in &policy.issuers {
            if !membership.accounts.contains_key(&rule.issuer) {
                return Err(SuppliesRejection::Policy.into());
            }
        }
        for rule in &policy.burners {
            if !membership.accounts.contains_key(&rule.issuer) {
                return Err(SuppliesRejection::Policy.into());
            }
        }
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        schema::initialize(&mut connection, scope, policy, membership)?;
        Ok(Self {
            connection,
            policy: policy.clone(),
            auth: AuthRuntime::new()?,
            quarantined: false,
        })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        policy: &SuppliesPolicy,
        known: KnownSuppliesFrontiers,
    ) -> Result<Self> {
        policy.validate()?;
        let length = std::fs::metadata(path.as_ref())
            .map_err(|_| crate::StoreError::MissingHistory)?
            .len();
        if !(100..=268435456).contains(&length) {
            return Err(SuppliesStoreError::Corrupt);
        }
        let scope = Scope {
            universe: policy.universe,
            history: policy.history,
        };
        if known.scope != scope {
            return Err(SuppliesRejection::Scope.into());
        }
        let mut connection = crate::schema::connection(path.as_ref())?;
        let tx = connection.transaction()?;
        let revision = ledger::verify(&tx, policy)?.revision;
        let membership = membership::load(&tx, scope)?;
        if revision < known.revision || membership.revision < known.membership_revision {
            return Err(SuppliesStoreError::StaleBackup);
        }
        tx.commit()?;
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            policy: policy.clone(),
            auth: AuthRuntime::new()?,
            quarantined: false,
        })
    }
    pub(super) fn scope(&self) -> Scope {
        Scope {
            universe: self.policy.universe,
            history: self.policy.history,
        }
    }
    fn current(&self) -> Result<MembershipState> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        membership::load(&self.connection, self.scope())
    }
    pub fn known_frontiers(&self) -> Result<KnownSuppliesFrontiers> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let tx = self.connection.unchecked_transaction()?;
        let membership = membership::load(&tx, self.scope())?;
        let revision = ledger::verify(&tx, &self.policy)?.revision;
        tx.commit()?;
        Ok(KnownSuppliesFrontiers {
            scope: self.scope(),
            revision,
            membership_revision: membership.revision,
        })
    }
    pub fn issue_challenge(&mut self, request: ChallengeRequest<'_>) -> Result<IssuedChallenge> {
        let membership = self.current()?;
        self.auth
            .issue(request, &membership, policy_digest(&self.policy)?)
    }
    /// One-use authorization and all durable issuance effects share one immediate transaction.
    pub fn issue(
        &mut self,
        issuance: &Issuance,
        proof: ProofAttempt,
    ) -> Result<Option<IssuanceOutcome>> {
        let evidence = self.auth.take(proof)?;
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let membership = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Issue(issuance), &membership)?;
        self.policy.permit(issuance)?;
        if !membership.accounts.contains_key(&issuance.beneficiary) {
            return Err(SuppliesRejection::Unauthorized.into());
        }
        let state = ledger::verify(&tx, &self.policy)?;
        let revision = ledger::apply(&tx, ledger::Operation::Issue(*issuance), &state)?.revision;
        let outcome = IssuanceOutcome {
            issuance: issuance.issuance,
            revision,
        };
        auth::live(&evidence)?;
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            return Err(error.into());
        }
        auth::live(&evidence)?;
        Ok(Some(outcome))
    }
    /// Signed authority burn retains accepted and insufficient-stock decisions in one immediate transaction.
    pub fn burn(&mut self, burn: &Burn, proof: ProofAttempt) -> Result<Option<BurnOutcome>> {
        let evidence = self.auth.take(proof)?;
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let membership = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Burn(burn), &membership)?;
        self.policy.permit_burn(burn)?;
        if !membership.accounts.contains_key(&burn.owner) {
            return Err(SuppliesRejection::Unauthorized.into());
        }
        let state = ledger::verify(&tx, &self.policy)?;
        let entry = ledger::apply(&tx, ledger::Operation::Burn(*burn), &state)?;
        auth::live(&evidence)?;
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            return Err(error.into());
        }
        auth::live(&evidence)?;
        if entry.decision == ledger::Decision::InsufficientAvailable {
            return Err(SuppliesRejection::Limit.into());
        }
        Ok(Some(BurnOutcome {
            burn: burn.burn,
            revision: entry.revision,
        }))
    }
    /// Signed own-account balance from one fresh membership and ledger snapshot.
    pub fn balance(&mut self, query: &BalanceQuery, proof: ProofAttempt) -> Result<Balances> {
        let evidence = self.auth.take(proof)?;
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self.connection.transaction()?;
        let membership = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Balance(query), &membership)?;
        if query.universe != self.policy.universe || query.history != self.policy.history {
            return Err(SuppliesRejection::Scope.into());
        }
        if query.owner != query.actor {
            return Err(SuppliesRejection::Unauthorized.into());
        }
        if query.content != SUPPLIES_CONTENT {
            return Err(SuppliesRejection::UnsupportedContent.into());
        }
        let state = ledger::verify(&tx, &self.policy)?;
        let balance = state
            .balances
            .get(&(query.owner, query.content, query.origin))
            .copied()
            .unwrap_or_default();
        ledger::conservation(balance)?;
        let current = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Balance(query), &current)?;
        auth::live(&evidence)?;
        tx.commit()?;
        auth::live(&evidence)?;
        Ok(balance)
    }
    /// Authenticated own-stock lookup returns the original immutable journal outcome.
    pub fn status(
        &mut self,
        query: &StatusQuery,
        proof: ProofAttempt,
    ) -> Result<Option<RequestOutcome>> {
        let evidence = self.auth.take(proof)?;
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self.connection.transaction()?;
        let membership = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Status(query), &membership)?;
        if query.universe != self.policy.universe || query.history != self.policy.history {
            return Err(SuppliesRejection::Scope.into());
        }
        if query.owner != query.actor {
            return Err(SuppliesRejection::Unauthorized.into());
        }
        let state = ledger::verify(&tx, &self.policy)?;
        let outcome = if let Some((_, operation)) = state.requests.get(&query.request) {
            let entry = state
                .operations
                .get(operation)
                .ok_or(SuppliesStoreError::Corrupt)?;
            if entry.operation.key().0 != query.owner {
                return Err(SuppliesRejection::Unauthorized.into());
            }
            Some(
                if entry.decision == ledger::Decision::InsufficientAvailable {
                    RequestOutcome::Rejected(RejectedOutcome {
                        operation: entry.operation.id(),
                        revision: entry.revision,
                        reason: RequestRejection::InsufficientAvailable,
                    })
                } else {
                    match entry.operation {
                        ledger::Operation::Issue(issuance) => {
                            RequestOutcome::Issued(IssuanceOutcome {
                                issuance: issuance.issuance,
                                revision: entry.revision,
                            })
                        }
                        ledger::Operation::Burn(burn) => RequestOutcome::Burned(BurnOutcome {
                            burn: burn.burn,
                            revision: entry.revision,
                        }),
                        ledger::Operation::Reserve(reserve) => {
                            RequestOutcome::Reserved(ReserveOutcome {
                                reservation: reserve.reservation,
                                revision: entry.revision,
                            })
                        }
                    }
                },
            )
        } else {
            None
        };
        auth::live(&evidence)?;
        tx.commit()?;
        auth::live(&evidence)?;
        Ok(outcome)
    }
    /// Signed own-stock reservation retains accepted and insufficient-stock decisions before returning.
    pub fn reserve(
        &mut self,
        reserve: &Reserve,
        proof: ProofAttempt,
    ) -> Result<Option<ReserveOutcome>> {
        let evidence = self.auth.take(proof)?;
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let membership = membership::load(&tx, scope)?;
        auth::verify(&evidence, ChallengeRequest::Reserve(reserve), &membership)?;
        if reserve.universe != self.policy.universe || reserve.history != self.policy.history {
            return Err(SuppliesRejection::Scope.into());
        }
        if reserve.policy != policy_digest(&self.policy)? {
            return Err(SuppliesRejection::Policy.into());
        }
        if reserve.owner != reserve.actor {
            return Err(SuppliesRejection::Unauthorized.into());
        }
        if reserve.content != SUPPLIES_CONTENT {
            return Err(SuppliesRejection::UnsupportedContent.into());
        }
        let state = ledger::verify(&tx, &self.policy)?;
        if reserve.amount == 0 {
            return Err(SuppliesRejection::Limit.into());
        }
        let entry = ledger::apply(&tx, ledger::Operation::Reserve(*reserve), &state)?;
        auth::live(&evidence)?;
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            return Err(error.into());
        }
        auth::live(&evidence)?;
        if entry.decision == ledger::Decision::InsufficientAvailable {
            return Err(SuppliesRejection::Limit.into());
        }
        Ok(Some(ReserveOutcome {
            reservation: reserve.reservation,
            revision: entry.revision,
        }))
    }
    /// A logical checkpoint preserves every original issuance, request and revision.
    pub fn compact(&mut self) -> Result<()> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        let scope = self.scope();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        membership::load(&tx, scope)?;
        let before = ledger::verify(&tx, &self.policy)?;
        super::compact::checkpoint(&tx, &before)?;
        let after = ledger::verify(&tx, &self.policy)?;
        if before != after {
            return Err(SuppliesStoreError::Corrupt);
        }
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            return Err(super::compact::denied(
                super::compact::CompactStage::CheckpointCommit,
                error,
            ));
        }
        Ok(())
    }
}
