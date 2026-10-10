use super::{
    ChallengeRequest, ChatPolicy, ChatReceipt, ChatStoreError, HistoryPage, HistoryQuery,
    IssuedChallenge, KnownChatFrontiers, LocalReceiptIssuer, ProofAttempt, Result, SignedMessage,
    auth::AuthRuntime, codec, history, membership, receipt_issuer, schema, write,
};
use nf_contract::identity::RequestId;
use nf_identity::model::{AdmissionProof, DeviceRevocation, MembershipState, SignedInvitation};
use rusqlite::{Connection, TransactionBehavior};
use std::path::{Path, PathBuf};
pub struct ChatStore {
    connection: Connection,
    path: PathBuf,
    policy: ChatPolicy,
    auth: AuthRuntime,
    quarantined: bool,
}
impl ChatStore {
    pub fn create(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        membership: &MembershipState,
    ) -> Result<Self> {
        nf_identity::codec::validate_state(membership)?;
        if membership.scope != policy.scope {
            return Err(ChatStoreError::Scope);
        }
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        schema::initialize(&mut connection, policy, membership)?;
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            policy: *policy,
            auth: AuthRuntime::new(),
            quarantined: false,
        })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        policy: &ChatPolicy,
        known: KnownChatFrontiers,
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
        let tx = connection.transaction()?;
        let state = schema::verify(&tx, policy)?;
        let membership = schema::current(&tx, policy.scope)?;
        if state.revision < known.revision || membership.revision < known.membership_revision {
            return Err(ChatStoreError::StaleBackup);
        }
        tx.commit()?;
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            policy: *policy,
            auth: AuthRuntime::new(),
            quarantined: false,
        })
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
        let next = membership::admit(&tx, &self.policy, signed, proof, local_now)?;
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
        let receipt = write::post(&tx, &self.policy, &state, request, message)?;
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
