use super::{
    ChallengeRequest, ChatPolicy, ChatRatePolicy, ChatReceipt, ChatStoreError, HistoryPage,
    HistoryQuery, IssuedChallenge, KnownChatFrontiers, LocalReceiptIssuer, MonotonicReceiverClock,
    ProofAttempt, ReceiverClock, Result, SignedMessage, auth::AuthRuntime, codec, history,
    membership, rate_runtime::Runtime, rate_sql, receipt_issuer, schema, write,
};
use nf_contract::identity::RequestId;
use nf_identity::model::{
    AdmissionProof, DeviceEnrollment, DeviceEnrollmentProof, DeviceRevocation, MembershipState,
    SignedInvitation,
};
use rusqlite::{Connection, TransactionBehavior};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
pub struct ChatStore {
    connection: Connection,
    path: PathBuf,
    policy: ChatPolicy,
    auth: AuthRuntime,
    rate: Runtime,
    quarantined: bool,
}
impl ChatStore {
    pub fn create(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        Self::create_with_rate_policy_and_clock(
            path,
            policy,
            membership,
            ChatRatePolicy::default(),
            Arc::new(MonotonicReceiverClock::default()),
        )
    }
    /// Trusted LOCAL clock seam retained from the approved deterministic FIRST.
    pub fn create_with_clock(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        membership: &MembershipState,
        clock: Arc<dyn ReceiverClock>,
    ) -> Result<Self> {
        Self::create_with_rate_policy_and_clock(
            path,
            policy,
            membership,
            ChatRatePolicy::default(),
            clock,
        )
    }
    /// Trusted LOCAL receiver policy and clock port; never decoded from a peer frame.
    pub fn create_with_rate_policy_and_clock(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        membership: &MembershipState,
        rate: ChatRatePolicy,
        clock: Arc<dyn ReceiverClock>,
    ) -> Result<Self> {
        nf_identity::codec::validate_state(membership)?;
        if membership.scope != policy.scope {
            return Err(ChatStoreError::Scope);
        }
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        schema::initialize(&mut connection, policy, membership, rate)?;
        let snapshot = rate_sql::verify_all(&connection, membership)?;
        let rate = Runtime::restore(&snapshot, clock);
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            policy: *policy,
            auth: AuthRuntime::new(),
            rate,
            quarantined: false,
        })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        known: KnownChatFrontiers,
    ) -> Result<Self> {
        Self::open_existing_with_clock(
            path,
            policy,
            known,
            Arc::new(MonotonicReceiverClock::default()),
        )
    }
    /// Reopening restores only committed balance and starts fresh local clock baselines.
    pub fn open_existing_with_clock(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        known: KnownChatFrontiers,
        clock: Arc<dyn ReceiverClock>,
    ) -> Result<Self> {
        if known.scope != policy.scope {
            return Err(ChatStoreError::Scope);
        }
        let metadata =
            std::fs::metadata(path.as_ref()).map_err(|_| ChatStoreError::MissingHistory)?;
        if !metadata.is_file() || !(100..=268435456).contains(&metadata.len()) {
            return Err(ChatStoreError::Corrupt);
        }
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        let tx = connection.transaction()?;
        let state = schema::verify(&tx, policy)?;
        let membership = schema::current(&tx, policy.scope)?;
        if state.revision < known.revision || membership.revision < known.membership_revision {
            return Err(ChatStoreError::StaleBackup);
        }
        let snapshot = rate_sql::verify_all(&tx, &membership)?;
        tx.commit()?;
        let rate = Runtime::restore(&snapshot, clock);
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            policy: *policy,
            auth: AuthRuntime::new(),
            rate,
            quarantined: false,
        })
    }
    /// Explicit LOCAL v2 migration. Ordinary open never upgrades an existing history.
    /// Existing accounts start empty; callers must separately authorize real-store migration.
    pub fn migrate_v2_with_rate_policy(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        known: KnownChatFrontiers,
        rate: ChatRatePolicy,
        clock: Arc<dyn ReceiverClock>,
    ) -> Result<Self> {
        let metadata =
            std::fs::metadata(path.as_ref()).map_err(|_| ChatStoreError::MissingHistory)?;
        if !metadata.is_file() || !(100..=268435456).contains(&metadata.len()) {
            return Err(ChatStoreError::Corrupt);
        }
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        let snapshot = schema::migrate_v2(&mut connection, policy, known, rate)?;
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            policy: *policy,
            auth: AuthRuntime::new(),
            rate: Runtime::restore(&snapshot, clock),
            quarantined: false,
        })
    }
    #[cfg(test)]
    pub(super) fn atomic_test_snapshot(
        &self,
        account: nf_contract::identity::AccountId,
    ) -> Result<(usize, usize, u16, u64)> {
        let state = schema::verify(&self.connection, &self.policy)?;
        let balance = rate_sql::balance(
            &self.connection,
            self.policy.scope,
            account,
            self.rate.policy(),
            self.rate.generation(),
        )?;
        Ok((
            state.requests.len(),
            state.messages.len(),
            balance.tokens,
            balance.fractional_ns,
        ))
    }
    fn live(&self) -> Result<()> {
        live_path(&self.path, self.quarantined)
    }
    pub fn known_frontiers(&self) -> Result<KnownChatFrontiers> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = schema::verify(&tx, &self.policy)?;
        let membership = schema::current(&tx, self.policy.scope)?;
        tx.commit()?;
        self.live()?;
        Ok(KnownChatFrontiers {
            scope: self.policy.scope,
            revision: state.revision,
            membership_revision: membership.revision,
        })
    }
    /// Verified current stored membership snapshot for trusted LOCAL foreground policy checks.
    /// Read-only: no membership enrollment, ticket consumption, or history/frontier mutation.
    pub fn current_membership(&self) -> Result<MembershipState> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        tx.commit()?;
        self.live()?;
        Ok(current)
    }
    /// Trusted LOCAL signing port. No receipt fields or public keys are accepted from a peer.
    /// Derives delivery evidence from the first durable original and never advances history.
    pub fn issue_delivery_receipt(
        &mut self,
        original_request: RequestId,
        issuer: LocalReceiptIssuer<'_>,
    ) -> Result<super::outbox::SignedChatReceipt> {
        self.live()?;
        let tx = self.connection.transaction()?;
        let state = schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        let receipt =
            receipt_issuer::issue(&self.policy, &current, &state, original_request, &issuer)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(receipt)
    }
    /// Applies the existing account-signed revocation policy; never a raw remote state commit.
    pub fn revoke_device(
        &mut self,
        change: &DeviceRevocation,
        signature: &[u8; 64],
    ) -> Result<MembershipState> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let next = membership::revoke(&tx, &self.policy, change, signature)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(next)
    }
    /// Trusted LOCAL account-signed device insertion; no roles, buckets or old keys change.
    pub fn enroll_device(
        &mut self,
        change: &DeviceEnrollment,
        proof: &DeviceEnrollmentProof,
    ) -> Result<MembershipState> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        let snapshot = rate_sql::verify_all(&tx, &current)?;
        self.rate.prepare_reload(snapshot, self.rate.policy())?;
        let next = membership::enroll(&tx, &self.policy, &current, change, proof)?;
        schema::verify(&tx, &self.policy)?;
        rate_sql::verify_all(&tx, &next)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(next)
    }
    /// Local supervisor entrypoint: local_now must come from trusted host policy, never remote input.
    /// Accepts only an issuer-signed PLAYER invitation and the recipient's two genuine proofs.
    pub fn admit_player(
        &mut self,
        signed: &SignedInvitation,
        proof: &AdmissionProof,
        local_now: u64,
    ) -> Result<MembershipState> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous = schema::current(&tx, self.policy.scope)?;
        let next = membership::admit(&tx, &self.policy, signed, proof, local_now)?;
        let snapshot = rate_sql::verify_all(&tx, &previous)?;
        self.rate.prepare_reload(snapshot, self.rate.policy())?;
        let (added, cut) = self.rate.new_members(&next)?;
        rate_sql::add_accounts(
            &tx,
            self.policy.scope,
            &added,
            self.rate.policy(),
            self.rate.generation(),
        )?;
        rate_sql::verify_all(&tx, &next)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.rate.commit_members(added, cut) {
            self.quarantined = true;
            return Err(error);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(next)
    }
    /// Read-only trusted LOCAL receiver policy snapshot; no sender-controlled override.
    pub fn receiver_rate_policy(&self) -> Result<(ChatRatePolicy, u64)> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        schema::verify(&tx, &self.policy)?;
        let actual = rate_sql::meta(&tx, self.policy.scope)?;
        if actual != (self.rate.policy(), self.rate.generation()) {
            return Err(ChatStoreError::Corrupt);
        }
        tx.commit()?;
        self.live()?;
        Ok(actual)
    }
    /// Compare-and-swap trusted LOCAL policy reload. Earned old-policy time is settled once;
    /// neither an unchanged reload nor reopening fills buckets.
    pub fn reload_receiver_rate_policy(
        &mut self,
        expected_generation: u64,
        next: ChatRatePolicy,
    ) -> Result<u64> {
        self.live()?;
        if expected_generation != self.rate.generation() {
            return Err(ChatStoreError::Conflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        schema::verify(&tx, &self.policy)?;
        let members = schema::current(&tx, self.policy.scope)?;
        let snapshot = rate_sql::verify_all(&tx, &members)?;
        let plan = self.rate.prepare_reload(snapshot, next)?;
        if let Some(plan) = &plan {
            rate_sql::reload(
                &tx,
                self.policy.scope,
                self.rate.policy(),
                self.rate.generation(),
                plan,
            )?;
        }
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Some(plan) = plan {
            self.rate.commit_reload(plan);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(self.rate.generation())
    }
    pub fn issue_challenge(
        &mut self,
        request: ChallengeRequest<'_>,
        peer: &[u8],
    ) -> Result<IssuedChallenge> {
        self.live()?;
        let tx = self.connection.transaction()?;
        schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        let (actor, binding) = match request {
            ChallengeRequest::Post { request, message } => {
                codec::verify_message(message, &current)?;
                (
                    message.message.author,
                    codec::post_binding(&self.policy, request, message)?,
                )
            }
            ChallengeRequest::History(query) => {
                (query.reader, codec::history_binding(&self.policy, query)?)
            }
        };
        tx.commit()?;
        self.live()?;
        self.auth.issue(&current, actor, peer, binding)
    }
    pub fn post(
        &mut self,
        request: RequestId,
        message: &SignedMessage,
        attempt: ProofAttempt<'_>,
    ) -> Result<ChatReceipt> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        let binding = codec::post_binding(&self.policy, request, message);
        self.auth
            .consume(&current, message.message.author, binding, attempt)?;
        codec::verify_message(message, &current)?;
        let prepared = write::prepare(&self.policy, &state, request, message)?;
        let debit = if prepared.is_fresh() {
            let before = rate_sql::balance(
                &tx,
                self.policy.scope,
                message.message.author.account,
                self.rate.policy(),
                self.rate.generation(),
            )?;
            Some(self.rate.prepare(message.message.author.account, before)?)
        } else {
            None
        };
        #[cfg(test)]
        if debit.is_some() {
            super::rate_crash_controls::cut("post-before-debit");
        }
        if let Some(debit) = &debit {
            rate_sql::debit(&tx, self.policy.scope, self.rate.generation(), debit)?;
        }
        #[cfg(test)]
        if debit.is_some() {
            super::rate_crash_controls::cut("post-after-debit");
        }
        let receipt = write::apply(&tx, &state, request, message, &prepared)?;
        live_path(&self.path, self.quarantined)?;
        #[cfg(test)]
        if debit.is_some() {
            super::rate_crash_controls::cut("post-before-commit");
        }
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        #[cfg(test)]
        if debit.is_some() {
            super::rate_crash_controls::cut("post-after-commit");
        }
        if let Some(debit) = debit
            && let Err(error) = self.rate.commit(debit)
        {
            self.quarantined = true;
            return Err(error);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(receipt)
    }
    pub fn history(
        &mut self,
        query: &HistoryQuery,
        attempt: ProofAttempt<'_>,
    ) -> Result<HistoryPage> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = schema::verify(&tx, &self.policy)?;
        let current = schema::current(&tx, self.policy.scope)?;
        self.auth.consume(
            &current,
            query.reader,
            codec::history_binding(&self.policy, query),
            attempt,
        )?;
        let page = history::read(&state, query)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        self.live()?;
        Ok(page)
    }
}

fn live_path(path: &Path, quarantined: bool) -> Result<()> {
    if quarantined {
        return Err(ChatStoreError::Quarantined);
    }
    let metadata = std::fs::metadata(path).map_err(|_| ChatStoreError::MissingHistory)?;
    if !metadata.is_file() || metadata.len() > 268435456 {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(())
}
