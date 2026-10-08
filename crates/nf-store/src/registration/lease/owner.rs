use super::{
    IssuedChallenge, ProofAttempt,
    auth::{self, AuthRuntime},
    model::*,
    replay, write,
};
use crate::registration::{KnownRegistrationFrontier, RegistrationPolicy};
use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;

/// Explicit trusted headless owner; profile-one registration remains a separate API.
/// Profile two fixes its registration binding set; this owner cannot register new branches.
pub struct LeaseStore {
    connection: Connection,
    registration_policy: RegistrationPolicy,
    admission_policy: AdmissionPolicy,
    known_registration: KnownRegistrationFrontier,
    known_admission: KnownAdmissionFrontier,
    auth: AuthRuntime,
    quarantined: bool,
}
impl LeaseStore {
    /// Verifies both histories first. Valid opens configure durability/locking PRAGMAs,
    /// but never execute DDL, migrate a profile, or create a grant.
    pub fn open_existing(
        path: impl AsRef<Path>,
        registration_policy: &RegistrationPolicy,
        admission_policy: &AdmissionPolicy,
        known_registration: KnownRegistrationFrontier,
        known_admission: KnownAdmissionFrontier,
    ) -> Result<Self> {
        admission_policy.validate(registration_policy)?;
        if known_registration.scope != registration_policy.scope
            || known_admission.scope != registration_policy.scope
        {
            return Err(LeaseError::Scope);
        }
        let length = std::fs::metadata(path.as_ref())
            .map_err(|_| crate::StoreError::MissingHistory)?
            .len();
        if !(100..=268435456).contains(&length) {
            return Err(LeaseError::Corrupt);
        }
        let auth = AuthRuntime::new()?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        let tx = connection.transaction()?;
        let state = replay::verify(&tx, registration_policy, admission_policy)?;
        replay::check_known(&state, known_registration, known_admission)?;
        tx.commit()?;
        // Refusals above preserve the database and sidecars; configuration is only for a valid owner.
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            registration_policy: registration_policy.clone(),
            admission_policy: *admission_policy,
            known_registration,
            known_admission,
            auth,
            quarantined: false,
        })
    }
    fn ensure(&self) -> Result<()> {
        if self.quarantined {
            return Err(crate::StoreError::Quarantined.into());
        }
        Ok(())
    }
    pub fn known_admission_frontier(&self) -> Result<KnownAdmissionFrontier> {
        self.ensure()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = replay::verify(&tx, &self.registration_policy, &self.admission_policy)?;
        replay::check_known(&state, self.known_registration, self.known_admission)?;
        tx.commit()?;
        Ok(state.frontier())
    }
    pub fn issue_admission_challenge(&mut self, request: &AdmitLease) -> Result<IssuedChallenge> {
        self.ensure()?;
        let tx = self.connection.transaction()?;
        let state = replay::verify(&tx, &self.registration_policy, &self.admission_policy)?;
        replay::check_known(&state, self.known_registration, self.known_admission)?;
        tx.commit()?;
        self.auth
            .issue(request, &state.membership, self.admission_policy.digest())
    }
    pub fn admit(&mut self, request: &AdmitLease, proof: ProofAttempt) -> Result<LeaseGranted> {
        let evidence = self.auth.take(proof)?;
        self.ensure()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = replay::verify(&tx, &self.registration_policy, &self.admission_policy)?;
        replay::check_known(&state, self.known_registration, self.known_admission)?;
        auth::verify(&evidence, request, &state.membership)?;
        replay::validate_request(
            request,
            &state,
            &self.registration_policy,
            &self.admission_policy,
        )?;
        auth::live(&evidence)?;
        let prepared = write::prepare(request, &state)?;
        let grant = prepared.grant();
        let frontier = prepared.frontier(&state);
        auth::live(&evidence)?;
        write::apply(
            &tx,
            &prepared,
            &state,
            &self.registration_policy,
            &self.admission_policy,
        )?;
        auth::live(&evidence)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(crate::StoreError::UncertainCommit.into());
        }
        // A committed prefix stays remembered even if the proof deadline expires before reply.
        self.known_admission = frontier;
        auth::live(&evidence)?;
        Ok(grant)
    }
}

impl nf_identity::model::MembershipRepository for LeaseStore {
    fn load_membership(
        &mut self,
        scope: nf_identity::model::Scope,
    ) -> std::result::Result<
        Option<nf_identity::model::MembershipState>,
        nf_identity::model::IdentityError,
    > {
        use nf_identity::model::IdentityError;
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        if scope != self.registration_policy.scope {
            return Err(IdentityError::Scope);
        }
        let tx = self
            .connection
            .transaction()
            .map_err(|_| IdentityError::Persistence)?;
        let state = replay::verify(&tx, &self.registration_policy, &self.admission_policy)
            .map_err(|_| IdentityError::Persistence)?;
        replay::check_known(&state, self.known_registration, self.known_admission)
            .map_err(|_| IdentityError::Persistence)?;
        tx.commit().map_err(|_| IdentityError::Persistence)?;
        Ok(Some(state.membership))
    }
    fn commit_membership(
        &mut self,
        expected_revision: Option<u64>,
        next: &nf_identity::model::MembershipState,
    ) -> std::result::Result<(), nf_identity::model::IdentityError> {
        use nf_identity::model::IdentityError;
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        crate::registration::membership::commit(
            &mut self.connection,
            &mut self.quarantined,
            self.registration_policy.scope,
            expected_revision,
            next,
        )
    }
}
