//! Owner-local read-only Chat status; no World, Nex, Market, submission or receipt-minting API.
use crate::{IpcError, LocalPrincipal, QueryPort};
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    model::{PublicIdentity, Roles, Scope},
    private_storage::PrivateVault,
};
use nf_store::chat::{
    ChatStore, KnownChatFrontiers,
    codec::encode_signed_message,
    outbox::{ClientOutbox, OutgoingState},
};
use nf_wire::generated as g;
pub struct ChatQueryPort {
    store: ChatStore,
    outbox: ClientOutbox,
    identity: PublicIdentity,
    scope: Scope,
    minimum_membership: u64,
}
impl ChatQueryPort {
    /// Trusted supervisor owns both files and chooses the full pinned local peer; wire fields never enroll keys.
    pub fn from_vault(
        store: ChatStore,
        outbox: ClientOutbox,
        vault: &PrivateVault,
        peer: Vec<u8>,
        minimum_membership: u64,
    ) -> Result<Self, IpcError> {
        let identity = vault
            .load_identity(peer)
            .map_err(|_| IpcError::PrivateStorage)?
            .public;
        let scope = store
            .known_frontiers()
            .map_err(|_| IpcError::ReadOnly)?
            .scope;
        let observed = authorize(&store, &identity, scope, minimum_membership)?;
        outbox.known_revision().map_err(|_| IpcError::ReadOnly)?;
        Ok(Self {
            store,
            outbox,
            identity,
            scope,
            minimum_membership: observed,
        })
    }
    pub fn principal(&self) -> LocalPrincipal {
        LocalPrincipal {
            account: self.identity.account,
            device: self.identity.device,
        }
    }
    /// Trusted owner handoff only. A decoded request cannot mutate either file through this port.
    pub fn into_parts(self) -> (ChatStore, ClientOutbox) {
        (self.store, self.outbox)
    }
    pub fn known_frontiers(&self) -> Result<KnownChatFrontiers, IpcError> {
        self.store.known_frontiers().map_err(|_| IpcError::ReadOnly)
    }
}
fn authorize(
    store: &ChatStore,
    identity: &PublicIdentity,
    scope: Scope,
    minimum: u64,
) -> Result<u64, IpcError> {
    let state = store.current_membership().map_err(|_| IpcError::ReadOnly)?;
    if state.scope != scope || state.revision < minimum {
        return Err(IpcError::ReadOnly);
    }
    let account = state
        .accounts
        .get(&identity.account)
        .ok_or(IpcError::Unauthorized)?;
    let device = state
        .devices
        .get(&identity.device)
        .ok_or(IpcError::Unauthorized)?;
    if device.revoked
        || device.account != identity.account
        || device.key != identity.device_key
        || device.peer != identity.peer
        || account.key != identity.account_key
        || !account.roles.contains(Roles::PLAYER)
    {
        return Err(IpcError::Unauthorized);
    }
    Ok(state.revision)
}
impl QueryPort for ChatQueryPort {
    fn query(&mut self, _: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        Err(IpcError::Unsupported)
    }
    fn query_chat_outgoing(
        &mut self,
        query: g::QueryChatOutgoing,
    ) -> Result<g::ChatOutgoingStatus, IpcError> {
        nf_wire::validate_query_chat_outgoing(&query).map_err(crate::session::wire_error)?;
        let principal = query.principal.as_ref().ok_or(IpcError::Unauthorized)?;
        if principal
            .account_id
            .as_ref()
            .ok_or(IpcError::Unauthorized)?
            .value
            != self.identity.account.as_bytes()
            || principal
                .device_id
                .as_ref()
                .ok_or(IpcError::Unauthorized)?
                .value
                != self.identity.device.as_bytes()
        {
            return Err(IpcError::Unauthorized);
        }
        let scope = Scope {
            universe: UniverseId::from_slice(
                &query.universe_id.as_ref().ok_or(IpcError::Malformed)?.value,
            )
            .map_err(|_| IpcError::Malformed)?,
            history: HistoryId::from_slice(
                &query.history_id.as_ref().ok_or(IpcError::Malformed)?.value,
            )
            .map_err(|_| IpcError::Malformed)?,
        };
        if scope != self.scope {
            return Err(IpcError::HistoryMismatch);
        }
        self.minimum_membership =
            authorize(&self.store, &self.identity, scope, self.minimum_membership)?;
        let message: [u8; 16] = query
            .message_id
            .as_ref()
            .ok_or(IpcError::Malformed)?
            .value
            .as_slice()
            .try_into()
            .map_err(|_| IpcError::Malformed)?;
        let entry = self
            .outbox
            .entry(message)
            .map_err(|_| IpcError::ReadOnly)?
            .ok_or(IpcError::UnknownOperation)?;
        if entry.signed.message.scope != scope
            || entry.signed.message.author.account != self.identity.account
            || entry.signed.message.author.device != self.identity.device
        {
            return Err(IpcError::Unauthorized);
        }
        if entry.original_request.as_bytes()
            != query
                .original_request_id
                .as_ref()
                .ok_or(IpcError::Malformed)?
                .value
                .as_slice()
        {
            return Err(IpcError::Malformed);
        }
        let revision = self
            .outbox
            .known_revision()
            .map_err(|_| IpcError::ReadOnly)?;
        let signed_message =
            encode_signed_message(&entry.signed).map_err(|_| IpcError::ReadOnly)?;
        let (phase, receiver_receipt) = match &entry.state {
            OutgoingState::Pending => (g::ChatOutgoingPhase::Pending, Vec::new()),
            OutgoingState::Delivered(receipt) => (
                g::ChatOutgoingPhase::Delivered,
                receipt
                    .to_canonical_bytes()
                    .map_err(|_| IpcError::ReadOnly)?,
            ),
        };
        let status = g::ChatOutgoingStatus {
            request_id: query.request_id,
            principal: query.principal,
            universe_id: query.universe_id,
            history_id: query.history_id,
            original_request_id: query.original_request_id,
            message_id: query.message_id,
            source_sequence: entry.signed.message.sequence,
            outbox_revision: revision,
            phase: phase as i32,
            signed_message,
            receiver_receipt,
        };
        nf_wire::validate_chat_outgoing_status(&status).map_err(crate::session::wire_error)?;
        Ok(status)
    }
}
