use super::{
    IssuedChallenge, ProofAttempt,
    auth::{self, AuthRuntime},
    membership,
    model::*,
    replay, schema, write,
};
use nf_identity::model::{MembershipState, Scope};
use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;

pub struct BranchRegistrar {
    pub(super) connection: Connection,
    policy: RegistrationPolicy,
    auth: AuthRuntime,
    pub(super) quarantined: bool,
}
impl BranchRegistrar {
    /// Explicit trusted headless initialization. No economic/native admission is created.
    pub fn create(
        path: impl AsRef<Path>,
        policy: &RegistrationPolicy,
        founder_membership: &MembershipState,
    ) -> Result<Self> {
        policy.validate()?;
        nf_identity::codec::validate_state(founder_membership)?;
        if founder_membership.scope != policy.scope {
            return Err(RegistrationError::Scope);
        }
        let auth = AuthRuntime::new()?;
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        schema::initialize(&mut connection, policy, founder_membership)?;
        Ok(Self {
            connection,
            policy: policy.clone(),
            auth,
            quarantined: false,
        })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        policy: &RegistrationPolicy,
        known: KnownRegistrationFrontier,
    ) -> Result<Self> {
        policy.validate()?;
        if known.scope != policy.scope {
            return Err(RegistrationError::Scope);
        }
        let length = std::fs::metadata(path.as_ref())
            .map_err(|_| crate::StoreError::MissingHistory)?
            .len();
        if !(100..=268435456).contains(&length) {
            return Err(RegistrationError::Corrupt);
        }
        let auth = AuthRuntime::new()?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        let tx = connection.transaction()?;
        let state = replay::verify(&tx, policy)?;
        let membership = membership::load(&tx, policy.scope)?;
        replay::check_known(&state, known, membership.revision)?;
        tx.commit()?;
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            policy: policy.clone(),
            auth,
            quarantined: false,
        })
    }
    pub(super) fn scope(&self) -> Scope {
        self.policy.scope
    }
    pub(super) fn ensure(&self) -> Result<()> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        Ok(())
    }
    pub fn known_frontier(&self) -> Result<KnownRegistrationFrontier> {
        self.ensure()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = replay::verify(&tx, &self.policy)?;
        let membership = membership::load(&tx, self.scope())?;
        tx.commit()?;
        Ok(KnownRegistrationFrontier {
            scope: self.scope(),
            revision: state.revision,
            head: state.head,
            minimum_membership_revision: membership.revision,
        })
    }
    pub fn issue_registration_challenge(
        &mut self,
        request: &RegisterBranch,
    ) -> Result<IssuedChallenge> {
        self.ensure()?;
        let tx = self.connection.transaction()?;
        replay::verify(&tx, &self.policy)?;
        let membership = membership::load(&tx, self.policy.scope)?;
        tx.commit()?;
        self.auth.issue(request, &membership, self.policy.digest()?)
    }
    pub fn register_branch(
        &mut self,
        request: &RegisterBranch,
        proof: ProofAttempt,
    ) -> Result<RegisteredBranch> {
        let evidence = self.auth.take(proof)?;
        self.ensure()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = replay::verify(&tx, &self.policy)?;
        let membership = membership::load(&tx, self.policy.scope)?;
        auth::verify(&evidence, request, &membership)?;
        self.policy.permit(request)?;
        auth::live(&evidence)?;
        let registered = write::register(&tx, request, &state)?;
        auth::live(&evidence)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(crate::StoreError::UncertainCommit.into());
        }
        auth::live(&evidence)?;
        Ok(registered)
    }
}
