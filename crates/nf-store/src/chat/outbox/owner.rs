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
