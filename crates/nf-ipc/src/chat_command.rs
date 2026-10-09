//! Trusted local Chat submission component; current local authority and a durable signed original.
use crate::{IpcError, LocalPrincipal, QueryPort};
use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::SecretSeed,
    model::{PublicIdentity, Roles, Scope},
    private_storage::PrivateVault,
};
use nf_store::chat::{
    ChatPolicy, ChatStore, ChatStoreError,
    codec::encode_signed_message,
    outbox::{ClientOutbox, OutgoingState},
};
use nf_wire::generated as g;
pub struct ChatCommandPort {
    store: ChatStore,
    outbox: ClientOutbox,
    identity: PublicIdentity,
    device_key: SecretSeed,
    policy: ChatPolicy,
    minimum_membership: u64,
}
impl ChatCommandPort {
    pub fn from_vault(
        store: ChatStore,
        outbox: ClientOutbox,
        vault: &PrivateVault,
        peer: Vec<u8>,
        minimum_membership: u64,
    ) -> Result<Self, IpcError> {
        let local = vault
            .load_identity(peer)
            .map_err(|_| IpcError::PrivateStorage)?;
        let policy = ChatPolicy {
            scope: store
                .known_frontiers()
                .map_err(|_| IpcError::ReadOnly)?
                .scope,
        };
        let observed = authorize(&store, &local.public, policy.scope, minimum_membership)?;
        outbox
            .validate_local_sender(&policy, &local.public)
            .map_err(outbox_error)?;
        Ok(Self {
            store,
            outbox,
            identity: local.public,
            device_key: local.device_key,
            policy,
            minimum_membership: observed,
        })
    }
    pub fn principal(&self) -> LocalPrincipal {
        LocalPrincipal {
            account: self.identity.account,
            device: self.identity.device,
        }
    }
    /// Trusted handoff only, after NodeServer lifecycle invalidation.
    pub fn into_parts(self) -> (ChatStore, ClientOutbox) {
        (self.store, self.outbox)
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
fn outbox_error(error: ChatStoreError) -> IpcError {
    match error {
        ChatStoreError::Signature => IpcError::Unauthorized,
        ChatStoreError::Policy | ChatStoreError::Scope => IpcError::PolicyMismatch,
        ChatStoreError::Malformed | ChatStoreError::Conflict => IpcError::Malformed,
        ChatStoreError::Limit => IpcError::Limit,
        _ => IpcError::ReadOnly,
    }
}
impl QueryPort for ChatCommandPort {
    fn query(&mut self, _: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        Err(IpcError::Unsupported)
    }
    fn enqueue_chat(&mut self, command: g::EnqueueChat) -> Result<g::ChatOutgoingStatus, IpcError> {
        nf_wire::validate_enqueue_chat(&command).map_err(crate::session::wire_error)?;
        let principal = command.principal.as_ref().ok_or(IpcError::Unauthorized)?;
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
                &command
                    .universe_id
                    .as_ref()
                    .ok_or(IpcError::Malformed)?
                    .value,
            )
            .map_err(|_| IpcError::Malformed)?,
            history: HistoryId::from_slice(
                &command
                    .history_id
                    .as_ref()
                    .ok_or(IpcError::Malformed)?
                    .value,
            )
            .map_err(|_| IpcError::Malformed)?,
        };
        if scope != self.policy.scope {
            return Err(IpcError::HistoryMismatch);
        }
        self.minimum_membership =
            authorize(&self.store, &self.identity, scope, self.minimum_membership)?;
        if self.device_key.public_key() != self.identity.device_key {
            return Err(IpcError::Unauthorized);
        }
        self.outbox
            .validate_local_sender(&self.policy, &self.identity)
            .map_err(outbox_error)?;
        let request = RequestId::from_slice(
            &command
                .original_request_id
                .as_ref()
                .ok_or(IpcError::Malformed)?
                .value,
        )
        .map_err(|_| IpcError::Malformed)?;
        let (entry, revision) = self
            .outbox
            .enqueue_local(request, &command.text, &self.device_key)
            .map_err(outbox_error)?;
        let signed_message = encode_signed_message(&entry.signed).map_err(outbox_error)?;
        let (phase, receiver_receipt) = match &entry.state {
            OutgoingState::Pending => (g::ChatOutgoingPhase::Pending, Vec::new()),
            OutgoingState::Delivered(receipt) => (
                g::ChatOutgoingPhase::Delivered,
                receipt.to_canonical_bytes().map_err(outbox_error)?,
            ),
        };
        let status = g::ChatOutgoingStatus {
            request_id: command.request_id,
            principal: Some(g::Principal {
                account_id: Some(g::AccountId {
                    value: self.identity.account.as_bytes().to_vec(),
                }),
                device_id: Some(g::DeviceId {
                    value: self.identity.device.as_bytes().to_vec(),
                }),
            }),
            universe_id: Some(g::UniverseId {
                value: self.policy.scope.universe.as_bytes().to_vec(),
            }),
            history_id: Some(g::HistoryId {
                value: self.policy.scope.history.as_bytes().to_vec(),
            }),
            original_request_id: Some(g::RequestId {
                value: entry.original_request.as_bytes().to_vec(),
            }),
            message_id: Some(g::ChatMessageId {
                value: entry.signed.message.message.to_vec(),
            }),
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
