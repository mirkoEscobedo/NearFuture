use super::{
    IssuedChallenge, ProofAttempt,
    auth::{self, AuthRuntime},
    model::*,
    replay, write,
};
use crate::registration::lease::{AdmissionPolicy, KnownAdmissionFrontier};
use crate::registration::{KnownRegistrationFrontier, RegistrationPolicy};
use rusqlite::{Connection, TransactionBehavior};
use std::path::Path;

/// Explicit self-report owner, over an already committed lease binding set.
/// No native attestation, clean bootstrap or economic capability is granted.
pub struct TaintStore {
    connection: Connection,
    registration_policy: RegistrationPolicy,
    admission_policy: AdmissionPolicy,
    taint_policy: TaintPolicy,
    known_registration: KnownRegistrationFrontier,
    known_admission: KnownAdmissionFrontier,
    known_taint: KnownTaintFrontier,
    auth: AuthRuntime,
    // Uncertain storage outcome only; a recorded self-report does not quarantine reads.
    quarantined: bool,
}
impl TaintStore {
    /// Full selected histories/policies/prefixes are verified before configuration.
    /// Valid opens configure durability PRAGMAs but never activate or migrate a profile.
    pub fn open_existing(
        path: impl AsRef<Path>,
        registration_policy: &RegistrationPolicy,
        admission_policy: &AdmissionPolicy,
        taint_policy: &TaintPolicy,
        known_registration: KnownRegistrationFrontier,
        known_admission: KnownAdmissionFrontier,
        known_taint: KnownTaintFrontier,
    ) -> Result<Self> {
        taint_policy.validate(registration_policy, admission_policy)?;
        if known_registration.scope != registration_policy.scope
            || known_admission.scope != registration_policy.scope
            || known_taint.scope != registration_policy.scope
        {
            return Err(TaintError::Scope);
        }
        let length = std::fs::metadata(path.as_ref())
            .map_err(|_| crate::StoreError::MissingHistory)?
            .len();
        if !(100..=268435456).contains(&length) {
            return Err(TaintError::Corrupt);
        }
        let auth = AuthRuntime::new()?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        let tx = connection.transaction()?;
        let state = replay::verify(&tx, registration_policy, admission_policy, taint_policy)?;
        replay::check_known(&state, known_registration, known_admission, known_taint)?;
        tx.commit()?;
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            registration_policy: registration_policy.clone(),
            admission_policy: *admission_policy,
            taint_policy: taint_policy.clone(),
            known_registration,
            known_admission,
            known_taint,
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
    pub fn known_taint_frontier(&self) -> Result<KnownTaintFrontier> {
        self.ensure()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = replay::verify(
            &tx,
            &self.registration_policy,
            &self.admission_policy,
            &self.taint_policy,
        )?;
        replay::check_known(
            &state,
            self.known_registration,
            self.known_admission,
            self.known_taint,
        )?;
        tx.commit()?;
        Ok(state.frontier())
    }
    pub fn issue_taint_challenge(
        &mut self,
        request: &MarkProhibitedManifest,
    ) -> Result<IssuedChallenge> {
        self.ensure()?;
        let tx = self.connection.transaction()?;
        let state = replay::verify(
            &tx,
            &self.registration_policy,
            &self.admission_policy,
            &self.taint_policy,
        )?;
        replay::check_known(
            &state,
            self.known_registration,
            self.known_admission,
            self.known_taint,
        )?;
        tx.commit()?;
        self.auth.issue(
            request,
            &state.leases.membership,
            self.taint_policy.digest()?,
        )
    }
    pub fn mark_prohibited_manifest(
        &mut self,
        request: &MarkProhibitedManifest,
        proof: ProofAttempt,
    ) -> Result<TaintRecorded> {
        let evidence = self.auth.take(proof)?;
        self.ensure()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = replay::verify(
            &tx,
            &self.registration_policy,
            &self.admission_policy,
            &self.taint_policy,
        )?;
        replay::check_known(
            &state,
            self.known_registration,
            self.known_admission,
            self.known_taint,
        )?;
        auth::verify(&evidence, request, &state.leases.membership)?;
        replay::validate_request(
            request,
            &state,
            &self.registration_policy,
            &self.admission_policy,
            &self.taint_policy,
        )?;
        auth::live(&evidence)?;
        let prepared = write::prepare(request, &state)?;
        let receipt = prepared.receipt();
        let frontier = prepared.frontier(&state);
        auth::live(&evidence)?;
        write::apply(
            &tx,
            &prepared,
            &state,
            &self.registration_policy,
            &self.admission_policy,
            &self.taint_policy,
        )?;
        auth::live(&evidence)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(crate::StoreError::UncertainCommit.into());
        }
        self.known_taint = frontier;
        auth::live(&evidence)?;
        Ok(receipt)
    }
}
