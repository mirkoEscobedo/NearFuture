use crate::{IpcError, LocalPrincipal, QueryPort};
use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    model::{MembershipRepository, PublicIdentity, Roles, Scope},
    private_storage::PrivateVault,
};
use nf_store::{KnownFrontiers, RequestStatus, Store};
use nf_wire::generated as g;
/// Owner-local read port. Token auth is performed by NodeServer; account identity comes from vault.
/// Every query reads current durable membership and checks revocation, keys, role and frontier.
pub struct StoreQueryPort {
    store: Store,
    identity: PublicIdentity,
    scope: Scope,
    minimum_membership: u64,
}
impl StoreQueryPort {
    pub fn from_vault(
        mut store: Store,
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
        let observed = authorize(&mut store, &identity, scope, minimum_membership)?;
        Ok(Self {
            store,
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
    /// Trusted background authority composition only; never exposed by QueryPort or a decoded message.
    pub fn owner_store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    pub fn into_store(self) -> Store {
        self.store
    }
    pub fn known_frontiers(&self) -> Result<KnownFrontiers, IpcError> {
        self.store.known_frontiers().map_err(|_| IpcError::ReadOnly)
    }
}
fn authorize(
    store: &mut Store,
    identity: &PublicIdentity,
    scope: Scope,
    minimum: u64,
) -> Result<u64, IpcError> {
    let state = store
        .load_membership(scope)
        .map_err(|_| IpcError::ReadOnly)?
        .ok_or(IpcError::Unauthorized)?;
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
impl QueryPort for StoreQueryPort {
    fn query(&mut self, query: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        let p = query.principal.ok_or(IpcError::Unauthorized)?;
        if p.account_id.ok_or(IpcError::Unauthorized)?.value != self.identity.account.as_bytes()
            || p.device_id.ok_or(IpcError::Unauthorized)?.value != self.identity.device.as_bytes()
        {
            return Err(IpcError::Unauthorized);
        }
        let scope = Scope {
            universe: UniverseId::from_slice(&query.universe_id.ok_or(IpcError::Malformed)?.value)
                .map_err(|_| IpcError::Malformed)?,
            history: HistoryId::from_slice(&query.history_id.ok_or(IpcError::Malformed)?.value)
                .map_err(|_| IpcError::Malformed)?,
        };
        if scope != self.scope {
            return Err(IpcError::HistoryMismatch);
        }
        self.minimum_membership = authorize(
            &mut self.store,
            &self.identity,
            scope,
            self.minimum_membership,
        )?;
        let request = RequestId::from_slice(&query.request_id.ok_or(IpcError::Malformed)?.value)
            .map_err(|_| IpcError::Malformed)?;
        let retained = self
            .store
            .query_bound(request, self.identity.account, self.identity.device, scope)
            .map_err(|_| IpcError::ReadOnly)?
            .ok_or(IpcError::UnknownOperation)?;
        let (operation, phase, outcome, sequence) = match retained.status {
            RequestStatus::Pending { operation } => {
                (operation, g::OperationPhase::Pending, None, None)
            }
            RequestStatus::Committed {
                operation,
                sequence,
                rejection: Some(rejection),
            } => (
                operation,
                g::OperationPhase::Rejected,
                Some(g::operation_status::Outcome::Error(g::BoundedError {
                    code: g::ErrorCode::RequestConflict as i32,
                    reason: format!("Retained kernel rejection: {rejection:?}."),
                    unsupported_capability_ids: vec![],
                    unsupported_schema_ids: vec![],
                    retryable: false,
                })),
                Some(g::EventSequence { value: sequence.0 }),
            ),
            // The foundation probe schema cannot represent a successful kernel operation outcome.
            RequestStatus::Committed {
                rejection: None, ..
            } => return Err(IpcError::Unsupported),
        };
        Ok(g::OperationStatus {
            request_id: Some(g::RequestId {
                value: request.as_bytes().to_vec(),
            }),
            operation_id: Some(g::OperationId {
                value: operation.as_bytes().to_vec(),
            }),
            history_id: Some(g::HistoryId {
                value: scope.history.as_bytes().to_vec(),
            }),
            request_binding_digest: Some(g::Sha256Digest {
                value: retained.binding_digest.to_vec(),
            }),
            phase: phase as i32,
            outcome,
            committed_event_seq: sequence,
        })
    }
}
