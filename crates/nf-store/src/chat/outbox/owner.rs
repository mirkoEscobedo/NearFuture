use super::{
    codec,
    model::{OutboxProfile, OutgoingEntry},
    receipt::{self, SignedChatReceipt},
    schema, write,
};
use crate::chat::{ChatStoreError, Result, SignedMessage};
use nf_contract::identity::RequestId;
use rusqlite::{Connection, TransactionBehavior};
use std::path::{Path, PathBuf};

pub struct ClientOutbox {
    connection: Connection,
    path: PathBuf,
    profile: OutboxProfile,
    quarantined: bool,
}
impl ClientOutbox {
    fn live(&self) -> Result<()> {
        live_path(&self.path, self.quarantined)
    }
    pub fn create(path: impl AsRef<Path>, profile: &OutboxProfile) -> Result<Self> {
        crate::schema::reserve(path.as_ref())?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        crate::schema::configure(&connection)?;
        schema::initialize(&mut connection, profile)?;
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            profile: profile.clone(),
            quarantined: false,
        })
    }
    pub fn open_existing(
        path: impl AsRef<Path>,
        profile: &OutboxProfile,
        known_revision: u64,
    ) -> Result<Self> {
        live_path(path.as_ref(), false)?;
        let mut connection = crate::schema::connection(path.as_ref())?;
        let tx = connection.transaction()?;
        let state = schema::verify(&tx, profile)?;
        if state.revision < known_revision {
            return Err(ChatStoreError::StaleBackup);
        }
        tx.commit()?;
        crate::schema::configure(&connection)?;
        Ok(Self {
            connection,
            path: path.as_ref().to_owned(),
            profile: profile.clone(),
            quarantined: false,
        })
    }
    pub fn known_revision(&self) -> Result<u64> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = schema::verify(&tx, &self.profile)?;
        tx.commit()?;
        self.live()?;
        Ok(state.revision)
    }
    pub fn entry(&self, message: [u8; 16]) -> Result<Option<OutgoingEntry>> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        let state = schema::verify(&tx, &self.profile)?;
        let entry = state.entries.get(&message).cloned();
        tx.commit()?;
        self.live()?;
        Ok(entry)
    }
    /// Read-only trusted local composition check. Wire data cannot enroll or replace lifetime pins.
    pub fn validate_local_sender(
        &self,
        policy: &crate::chat::ChatPolicy,
        identity: &nf_identity::model::PublicIdentity,
    ) -> Result<()> {
        self.live()?;
        let tx = self.connection.unchecked_transaction()?;
        schema::verify(&tx, &self.profile)?;
        if self.profile.policy != *policy {
            return Err(crate::chat::ChatStoreError::Policy);
        }
        if self.profile.sender.actor.account != identity.account
            || self.profile.sender.actor.device != identity.device
            || self.profile.sender.key != identity.device_key
            || self.profile.sender.peer != identity.peer
        {
            return Err(crate::chat::ChatStoreError::Signature);
        }
        tx.commit()?;
        self.live()
    }
    /// Allocate a trusted local original atomically; callers cannot supply a message ID or sequence.
    /// Exact retries return the retained original before capacity or allocation, including Delivered.
    pub fn enqueue_local(
        &mut self,
        request: RequestId,
        text: &str,
        device_key: &nf_identity::keys::SecretSeed,
    ) -> Result<(OutgoingEntry, u64)> {
        self.live()?;
        if request.as_bytes() == &[0; 16] || text.is_empty() {
            return Err(ChatStoreError::Malformed);
        }
        if text.len() > crate::chat::MAX_TEXT_BYTES {
            return Err(ChatStoreError::Limit);
        }
        if !nf_contract::canonical::text::is_canonical_text(text) {
            return Err(ChatStoreError::Malformed);
        }
        if device_key.public_key() != self.profile.sender.key {
            return Err(ChatStoreError::Signature);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = schema::verify(&tx, &self.profile)?;
        let (entry, revision) = if let Some(original) = state
            .entries
            .values()
            .find(|entry| entry.original_request == request)
        {
            if original.signed.message.author != self.profile.sender.actor
                || original.signed.message.scope != self.profile.policy.scope
                || original.signed.message.channel != crate::chat::Channel::General
                || original.signed.message.text != text
            {
                return Err(ChatStoreError::Conflict);
            }
            (original.clone(), state.revision)
        } else {
            if state.entries.len() >= 4096 {
                return Err(ChatStoreError::Limit);
            }
            let sequence = state
                .entries
                .values()
                .filter(|entry| entry.signed.message.author == self.profile.sender.actor)
                .map(|entry| entry.signed.message.sequence)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(ChatStoreError::Limit)?;
            let revision = state.revision.checked_add(1).ok_or(ChatStoreError::Limit)?;
            let message_id = nf_identity::keys::random_id()?;
            if message_id == [0; 16] || state.entries.contains_key(&message_id) {
                return Err(ChatStoreError::Conflict);
            }
            let message = crate::chat::ChatMessage {
                scope: self.profile.policy.scope,
                channel: crate::chat::Channel::General,
                author: self.profile.sender.actor,
                message: message_id,
                sequence,
                text: text.to_owned(),
            };
            let signature = device_key.sign(&crate::chat::codec::message_digest(&message)?);
            let signed = SignedMessage { message, signature };
            codec::validate(&self.profile, request, &signed)?;
            (write::enqueue(&tx, &state, request, &signed)?, revision)
        };
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok((entry, revision))
    }
    /// Only local outgoing originals: no remote membership, wire proof or financial authority.
    pub fn enqueue(&mut self, request: RequestId, signed: &SignedMessage) -> Result<OutgoingEntry> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = schema::verify(&tx, &self.profile)?;
        codec::validate(&self.profile, request, signed)?;
        let entry = write::enqueue(&tx, &state, request, signed)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(entry)
    }
    /// Durably retain a genuine receiver acknowledgment for the same immutable outgoing original.
    pub fn acknowledge(&mut self, signed: &SignedChatReceipt) -> Result<OutgoingEntry> {
        self.live()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = schema::verify(&tx, &self.profile)?;
        let original = state
            .entries
            .get(&signed.receipt.original.message)
            .ok_or(ChatStoreError::Conflict)?;
        codec::validate(&self.profile, original.original_request, &original.signed)?;
        receipt::validate(&self.profile, original, signed)?;
        let entry = write::acknowledge(&tx, &state, signed)?;
        live_path(&self.path, self.quarantined)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(ChatStoreError::Storage);
        }
        if let Err(error) = self.live() {
            self.quarantined = true;
            return Err(error);
        }
        Ok(entry)
    }
}
fn live_path(path: &Path, quarantined: bool) -> Result<()> {
    if quarantined {
        return Err(ChatStoreError::Quarantined);
    }
    let metadata = std::fs::metadata(path).map_err(|_| ChatStoreError::MissingHistory)?;
    if !metadata.is_file() || !(100..=268435456).contains(&metadata.len()) {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(())
}
