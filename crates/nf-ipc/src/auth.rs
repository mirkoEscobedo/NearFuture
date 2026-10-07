use crate::session::{wire_error, wire_limits};
use crate::{AuthenticatedSession, IpcError, LocalPrincipal, SecretToken, SessionConfig};
use hmac::{Hmac, Mac};
use nf_wire::{generated as g, local_auth as a};
use prost::Message;
use sha2::Sha256;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointRole {
    Control = 1,
    Bulk = 2,
}
#[derive(Clone)]
pub struct AuthBinding {
    pub config: SessionConfig,
    pub principal: LocalPrincipal,
    pub role: EndpointRole,
    pub port: u16,
}
impl AuthBinding {
    pub fn validate(&self) -> Result<(), IpcError> {
        self.config.validate()?;
        if self.port == 0 {
            return Err(IpcError::Malformed);
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub enum ProofStage {
    ServerChallenge = 1,
    ClientProof = 2,
    ServerFinished = 3,
}
/// Fixed 306-byte NF-IPC-AUTH-1 transcript. Protobuf reserialization is never authenticated.
pub fn auth_transcript(
    binding: &AuthBinding,
    offered: g::ResourceLimits,
    selected: g::ResourceLimits,
    client_nonce: [u8; 32],
    server_nonce: [u8; 32],
    stage: ProofStage,
) -> Result<Vec<u8>, IpcError> {
    binding.validate()?;
    let mut check = binding.config.clone();
    check.limits = offered;
    check.validate()?;
    check.limits = selected;
    check.validate()?;
    if selected != negotiate(binding.config.limits, offered) {
        return Err(IpcError::PolicyMismatch);
    }
    let c = &binding.config;
    let mut b = Vec::with_capacity(306);
    b.extend_from_slice(b"NF-IPC-AUTH-1\0");
    b.push(stage as u8);
    for n in [1u32, 2, 2, 1] {
        b.extend_from_slice(&n.to_le_bytes());
    }
    b.push(binding.role as u8);
    b.extend_from_slice(&binding.port.to_le_bytes());
    b.extend_from_slice(&c.runtime_session.to_le_bytes());
    for field in [
        &c.universe[..],
        &c.history[..],
        &c.ruleset[..],
        &c.content_policy[..],
        binding.principal.account.as_bytes(),
        binding.principal.device.as_bytes(),
        &client_nonce,
        &server_nonce,
    ] {
        b.extend_from_slice(field);
    }
    for limits in [offered, selected] {
        for n in [
            limits.control_frame_bytes,
            limits.chunk_bytes,
            limits.inflight_bytes,
            limits.inflight_items,
            limits.decoded_bytes,
            limits.collection_items,
            limits.nesting_depth,
        ] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(&limits.transfer_bytes.to_le_bytes());
    }
    Ok(b)
}
pub fn auth_proof(token: &SecretToken, transcript: &[u8]) -> Result<[u8; 32], IpcError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(token.private_bytes()).map_err(|_| IpcError::Malformed)?;
    mac.update(transcript);
    Ok(mac.finalize().into_bytes().into())
}
fn verify(token: &SecretToken, transcript: &[u8], proof: &[u8]) -> Result<(), IpcError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(token.private_bytes()).map_err(|_| IpcError::Malformed)?;
    mac.update(transcript);
    mac.verify_slice(proof).map_err(|_| IpcError::Unauthorized)
}
fn nonce() -> Result<[u8; 32], IpcError> {
    let mut n = [0; 32];
    getrandom::fill(&mut n).map_err(|_| IpcError::Io)?;
    Ok(n)
}
fn decode(bytes: &[u8], binding: &AuthBinding) -> Result<a::LocalAuthEnvelope, IpcError> {
    let value = nf_wire::decode_local_auth_with_limits(bytes, wire_limits(&binding.config.limits))
        .map_err(wire_error)?;
    if value
        .runtime_session
        .as_ref()
        .ok_or(IpcError::Malformed)?
        .value
        != binding.config.runtime_session
    {
        return Err(IpcError::SessionMismatch);
    }
    Ok(value)
}
fn envelope(session: u64, body: a::local_auth_envelope::Body) -> Vec<u8> {
    a::LocalAuthEnvelope {
        auth_version: 1,
        capability_id: 2,
        schema_id: 2,
        runtime_session: Some(g::RuntimeSession { value: session }),
        body: Some(body),
    }
    .encode_to_vec()
}
fn negotiate(l: g::ResourceLimits, r: g::ResourceLimits) -> g::ResourceLimits {
    g::ResourceLimits {
        control_frame_bytes: l.control_frame_bytes.min(r.control_frame_bytes),
        chunk_bytes: l.chunk_bytes.min(r.chunk_bytes),
        inflight_bytes: l.inflight_bytes.min(r.inflight_bytes),
        inflight_items: l.inflight_items.min(r.inflight_items),
        decoded_bytes: l.decoded_bytes.min(r.decoded_bytes),
        collection_items: l.collection_items.min(r.collection_items),
        nesting_depth: l.nesting_depth.min(r.nesting_depth),
        transfer_bytes: l.transfer_bytes.min(r.transfer_bytes),
    }
}
/// Owner-private server credential. No raw token is ever serialized onto the network.
pub struct Authenticator {
    binding: AuthBinding,
    token: SecretToken,
}
pub struct ServerChallenge {
    client_nonce: [u8; 32],
    server_nonce: [u8; 32],
    offered: g::ResourceLimits,
    selected: g::ResourceLimits,
    encoded: Vec<u8>,
}
impl ServerChallenge {
    pub fn challenge(&self) -> &[u8] {
        &self.encoded
    }
}
impl Authenticator {
    pub fn new(binding: AuthBinding, token: SecretToken) -> Result<Self, IpcError> {
        binding.validate()?;
        Ok(Self { binding, token })
    }
    pub(crate) fn matches_config(
        &self,
        c: &SessionConfig,
        principal: LocalPrincipal,
        role: EndpointRole,
        port: u16,
    ) -> bool {
        let a = &self.binding.config;
        a.runtime_session == c.runtime_session
            && a.universe == c.universe
            && a.history == c.history
            && a.ruleset == c.ruleset
            && a.content_policy == c.content_policy
            && a.limits == c.limits
            && self.binding.principal == principal
            && self.binding.role == role
            && self.binding.port == port
    }
    pub fn begin(&self, encoded: &[u8]) -> Result<ServerChallenge, IpcError> {
        let e = decode(encoded, &self.binding)?;
        let Some(a::local_auth_envelope::Body::Hello(h)) = e.body else {
            return Err(IpcError::Unauthorized);
        };
        if h.endpoint_role != self.binding.role as i32 {
            return Err(IpcError::Unauthorized);
        }
        let p = h.principal.ok_or(IpcError::Unauthorized)?;
        if p.account_id.ok_or(IpcError::Unauthorized)?.value
            != self.binding.principal.account.as_bytes()
            || p.device_id.ok_or(IpcError::Unauthorized)?.value
                != self.binding.principal.device.as_bytes()
        {
            return Err(IpcError::Unauthorized);
        }
        let client_nonce = h.client_nonce.try_into().map_err(|_| IpcError::Malformed)?;
        let server_nonce = nonce()?;
        let offered = h.offered_limits.ok_or(IpcError::Malformed)?;
        let selected = negotiate(self.binding.config.limits, offered);
        let transcript = auth_transcript(
            &self.binding,
            offered,
            selected,
            client_nonce,
            server_nonce,
            ProofStage::ServerChallenge,
        )?;
        let proof = auth_proof(&self.token, &transcript)?;
        let encoded = envelope(
            self.binding.config.runtime_session,
            a::local_auth_envelope::Body::Challenge(a::LocalAuthChallenge {
                server_nonce: server_nonce.to_vec(),
                selected_limits: Some(selected),
                server_proof: proof.to_vec(),
            }),
        );
        Ok(ServerChallenge {
            client_nonce,
            server_nonce,
            offered,
            selected,
            encoded,
        })
    }
    pub fn finish(
        &self,
        challenge: ServerChallenge,
        encoded: &[u8],
    ) -> Result<(AuthenticatedSession, Vec<u8>), IpcError> {
        let e = decode(encoded, &self.binding)?;
        let Some(a::local_auth_envelope::Body::Proof(proof)) = e.body else {
            return Err(IpcError::Unauthorized);
        };
        let transcript = auth_transcript(
            &self.binding,
            challenge.offered,
            challenge.selected,
            challenge.client_nonce,
            challenge.server_nonce,
            ProofStage::ClientProof,
        )?;
        verify(&self.token, &transcript, &proof.client_proof)?;
        let transcript = auth_transcript(
            &self.binding,
            challenge.offered,
            challenge.selected,
            challenge.client_nonce,
            challenge.server_nonce,
            ProofStage::ServerFinished,
        )?;
        let finished = auth_proof(&self.token, &transcript)?;
        let encoded = envelope(
            self.binding.config.runtime_session,
            a::local_auth_envelope::Body::Accepted(a::LocalAuthAccepted {
                server_finished_proof: finished.to_vec(),
            }),
        );
        Ok((
            AuthenticatedSession::from_config(&self.binding.config, challenge.selected),
            encoded,
        ))
    }
}
/// Consuming client stages enforce server proof before client proof and finished proof before active.
pub struct ClientHello {
    binding: AuthBinding,
    token: SecretToken,
    offered: g::ResourceLimits,
    client_nonce: [u8; 32],
    encoded: Vec<u8>,
}
pub struct ClientProof {
    binding: AuthBinding,
    token: SecretToken,
    offered: g::ResourceLimits,
    selected: g::ResourceLimits,
    client_nonce: [u8; 32],
    server_nonce: [u8; 32],
    encoded: Vec<u8>,
}
impl ClientHello {
    pub fn start(
        binding: AuthBinding,
        token: SecretToken,
        offered: g::ResourceLimits,
    ) -> Result<Self, IpcError> {
        binding.validate()?;
        let mut check = binding.config.clone();
        check.limits = offered;
        check.validate()?;
        let client_nonce = nonce()?;
        let encoded = envelope(
            binding.config.runtime_session,
            a::local_auth_envelope::Body::Hello(a::LocalAuthHello {
                client_nonce: client_nonce.to_vec(),
                endpoint_role: binding.role as i32,
                offered_limits: Some(offered),
                principal: Some(g::Principal {
                    account_id: Some(g::AccountId {
                        value: binding.principal.account.as_bytes().to_vec(),
                    }),
                    device_id: Some(g::DeviceId {
                        value: binding.principal.device.as_bytes().to_vec(),
                    }),
                }),
                protocols: Some(g::ProtocolRange {
                    minimum: 1,
                    maximum: 1,
                }),
            }),
        );
        Ok(Self {
            binding,
            token,
            offered,
            client_nonce,
            encoded,
        })
    }
    pub fn hello(&self) -> &[u8] {
        &self.encoded
    }
    pub fn respond(self, encoded: &[u8]) -> Result<ClientProof, IpcError> {
        let e = decode(encoded, &self.binding)?;
        let Some(a::local_auth_envelope::Body::Challenge(challenge)) = e.body else {
            return Err(IpcError::Unauthorized);
        };
        let selected = challenge.selected_limits.ok_or(IpcError::Malformed)?;
        let server_nonce = challenge
            .server_nonce
            .try_into()
            .map_err(|_| IpcError::Malformed)?;
        let transcript = auth_transcript(
            &self.binding,
            self.offered,
            selected,
            self.client_nonce,
            server_nonce,
            ProofStage::ServerChallenge,
        )?;
        verify(&self.token, &transcript, &challenge.server_proof)?;
        let transcript = auth_transcript(
            &self.binding,
            self.offered,
            selected,
            self.client_nonce,
            server_nonce,
            ProofStage::ClientProof,
        )?;
        let proof = auth_proof(&self.token, &transcript)?;
        let encoded = envelope(
            self.binding.config.runtime_session,
            a::local_auth_envelope::Body::Proof(a::LocalAuthProof {
                client_proof: proof.to_vec(),
            }),
        );
        Ok(ClientProof {
            binding: self.binding,
            token: self.token,
            offered: self.offered,
            selected,
            client_nonce: self.client_nonce,
            server_nonce,
            encoded,
        })
    }
}
impl ClientProof {
    pub fn proof(&self) -> &[u8] {
        &self.encoded
    }
    pub fn finish(self, encoded: &[u8]) -> Result<AuthenticatedSession, IpcError> {
        let e = decode(encoded, &self.binding)?;
        let Some(a::local_auth_envelope::Body::Accepted(accepted)) = e.body else {
            return Err(IpcError::Unauthorized);
        };
        let transcript = auth_transcript(
            &self.binding,
            self.offered,
            self.selected,
            self.client_nonce,
            self.server_nonce,
            ProofStage::ServerFinished,
        )?;
        verify(&self.token, &transcript, &accepted.server_finished_proof)?;
        Ok(AuthenticatedSession::from_config(
            &self.binding.config,
            self.selected,
        ))
    }
}
