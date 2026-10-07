use crate::IpcError;
use nf_wire::generated as g;
use std::fmt;
use zeroize::Zeroizing;
pub struct SecretToken(Zeroizing<[u8; 32]>);
impl SecretToken {
    pub fn generate() -> Result<Self, IpcError> {
        let mut bytes = Zeroizing::new([0; 32]);
        getrandom::fill(bytes.as_mut()).map_err(|_| IpcError::Io)?;
        Ok(Self(bytes))
    }
    /// For a token imported from an owner-private rendezvous record. Caller must erase source bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }
    pub(crate) fn private_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
impl fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretToken([REDACTED])")
    }
}
#[derive(Clone)]
pub struct SessionConfig {
    pub universe: [u8; 16],
    pub history: [u8; 16],
    pub runtime_session: u64,
    pub ruleset: [u8; 32],
    pub content_policy: [u8; 32],
    pub limits: g::ResourceLimits,
}
impl SessionConfig {
    pub(crate) fn validate(&self) -> Result<(), IpcError> {
        let v = &self.limits;
        let actual = [
            v.control_frame_bytes as u64,
            v.chunk_bytes as u64,
            v.inflight_bytes as u64,
            v.inflight_items as u64,
            v.decoded_bytes as u64,
            v.collection_items as u64,
            v.nesting_depth as u64,
            v.transfer_bytes,
        ];
        let hard = [
            1_048_576, 262_144, 16_777_216, 256, 1_048_576, 4096, 32, 67_108_864,
        ];
        if self.runtime_session == 0 || actual.iter().zip(hard).any(|(v, c)| *v == 0 || *v > c) {
            return Err(IpcError::Limit);
        }
        if v.chunk_bytes > v.control_frame_bytes
            || v.control_frame_bytes > v.decoded_bytes
            || v.control_frame_bytes > v.inflight_bytes
            || v.chunk_bytes as u64 > v.transfer_bytes
        {
            return Err(IpcError::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct AuthenticatedSession {
    runtime_session: u64,
    universe: [u8; 16],
    history: [u8; 16],
    pub(crate) limits: g::ResourceLimits,
}
impl AuthenticatedSession {
    pub fn limits(&self) -> g::ResourceLimits {
        self.limits
    }
    pub(crate) fn from_config(config: &SessionConfig, limits: g::ResourceLimits) -> Self {
        Self {
            runtime_session: config.runtime_session,
            universe: config.universe,
            history: config.history,
            limits,
        }
    }
    pub fn runtime_session(&self) -> u64 {
        self.runtime_session
    }
}
pub(crate) fn wire_error(error: nf_wire::WireError) -> IpcError {
    match error {
        nf_wire::WireError::Limit => IpcError::Limit,
        nf_wire::WireError::Unsupported => IpcError::Unsupported,
        _ => IpcError::Malformed,
    }
}
pub(crate) fn wire_limits(v: &g::ResourceLimits) -> nf_wire::Limits {
    nf_wire::Limits {
        frame_bytes: v.control_frame_bytes as usize,
        field_bytes: v.chunk_bytes as usize,
        collection_items: v.collection_items as usize,
        depth: v.nesting_depth as usize,
        decoded_bytes: v.decoded_bytes as usize,
        ..Default::default()
    }
}
use nf_contract::identity::{AccountId, DeviceId, RequestId};
/// Trusted composition input. Token possession alone must never select this principal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalPrincipal {
    pub account: AccountId,
    pub device: DeviceId,
}
/// The lifecycle owner invalidates this before load/save/exit, then installs a fresh session.
pub struct SessionFence {
    current: Option<u64>,
}
impl SessionFence {
    pub fn new(session: u64) -> Result<Self, IpcError> {
        if session == 0 {
            return Err(IpcError::SessionMismatch);
        }
        Ok(Self {
            current: Some(session),
        })
    }
    pub fn invalidate(&mut self) {
        self.current = None;
    }
    pub(crate) fn check(&self, session: u64) -> Result<(), IpcError> {
        match self.current {
            None => Err(IpcError::ReadOnly),
            Some(v) if v != session => Err(IpcError::SessionMismatch),
            Some(_) => Ok(()),
        }
    }
}
#[derive(Debug)]
pub enum AdmittedClient {
    Intent(g::Intent),
    Query(g::QueryOperation),
}
#[derive(Debug)]
pub enum AdmittedServer {
    Status(g::OperationStatus),
    Error(g::BoundedError),
}
impl AuthenticatedSession {
    /// Background response admission under the same exact session, requested operation and history.
    /// This is retained status only; it does not authorize projection or game application.
    pub fn admit_response(
        &self,
        bytes: &[u8],
        request: RequestId,
        fence: &SessionFence,
    ) -> Result<AdmittedServer, IpcError> {
        fence.check(self.runtime_session)?;
        let e = nf_wire::decode_control_with_limits(bytes, wire_limits(&self.limits))
            .map_err(wire_error)?;
        if e.runtime_session.ok_or(IpcError::Malformed)?.value != self.runtime_session {
            return Err(IpcError::SessionMismatch);
        }
        match e.body.ok_or(IpcError::Malformed)? {
            g::control_envelope::Body::OperationStatus(status) => {
                if status.history_id.as_ref().ok_or(IpcError::Malformed)?.value != self.history {
                    return Err(IpcError::HistoryMismatch);
                }
                if status.request_id.as_ref().ok_or(IpcError::Malformed)?.value
                    != request.as_bytes()
                {
                    return Err(IpcError::Malformed);
                }
                Ok(AdmittedServer::Status(status))
            }
            g::control_envelope::Body::Error(error) => Ok(AdmittedServer::Error(error)),
            _ => Err(IpcError::Unsupported),
        }
    }
    /// Background admission only. This does not verify signed intents or perform effects.
    pub fn admit_client(
        &self,
        bytes: &[u8],
        principal: LocalPrincipal,
        fence: &SessionFence,
    ) -> Result<AdmittedClient, IpcError> {
        fence.check(self.runtime_session)?;
        let e = nf_wire::decode_control_with_limits(bytes, wire_limits(&self.limits))
            .map_err(wire_error)?;
        if e.runtime_session.ok_or(IpcError::Malformed)?.value != self.runtime_session {
            return Err(IpcError::SessionMismatch);
        }
        let required = e.required.ok_or(IpcError::Malformed)?;
        if required.capability_ids != [1] || required.schema_ids != [1] {
            return Err(IpcError::Unsupported);
        }
        match e.body.ok_or(IpcError::Malformed)? {
            g::control_envelope::Body::Intent(v) => {
                self.check_principal(v.principal.as_ref(), principal)?;
                self.check_scope(v.universe_id.as_ref(), v.history_id.as_ref())?;
                Ok(AdmittedClient::Intent(v))
            }
            g::control_envelope::Body::QueryOperation(v) => {
                self.check_principal(v.principal.as_ref(), principal)?;
                self.check_scope(v.universe_id.as_ref(), v.history_id.as_ref())?;
                if v.request_id
                    .as_ref()
                    .ok_or(IpcError::Malformed)?
                    .value
                    .len()
                    != 16
                {
                    return Err(IpcError::Malformed);
                }
                Ok(AdmittedClient::Query(v))
            }
            _ => Err(IpcError::Unsupported),
        }
    }
    fn check_principal(
        &self,
        p: Option<&g::Principal>,
        trusted: LocalPrincipal,
    ) -> Result<(), IpcError> {
        let p = p.ok_or(IpcError::Unauthorized)?;
        if p.account_id.as_ref().ok_or(IpcError::Unauthorized)?.value != trusted.account.as_bytes()
            || p.device_id.as_ref().ok_or(IpcError::Unauthorized)?.value
                != trusted.device.as_bytes()
        {
            return Err(IpcError::Unauthorized);
        }
        Ok(())
    }
    fn check_scope(
        &self,
        u: Option<&g::UniverseId>,
        h: Option<&g::HistoryId>,
    ) -> Result<(), IpcError> {
        if u.ok_or(IpcError::Malformed)?.value != self.universe
            || h.ok_or(IpcError::Malformed)?.value != self.history
        {
            return Err(IpcError::HistoryMismatch);
        }
        Ok(())
    }
}
