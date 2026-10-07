use crate::{Limits, WireError, local_auth as a};
use prost::Message;
pub fn decode_local_auth(input: &[u8]) -> Result<a::LocalAuthEnvelope, WireError> {
    decode_local_auth_with_limits(input, Limits::default())
}
pub fn decode_local_auth_with_limits(
    input: &[u8],
    mut limits: Limits,
) -> Result<a::LocalAuthEnvelope, WireError> {
    limits.validate()?;
    limits.frame_bytes = limits.frame_bytes.min(4096);
    limits.decoded_bytes = limits.decoded_bytes.min(4096);
    limits.field_bytes = limits.field_bytes.min(4096);
    limits.depth = limits.depth.min(8);
    limits.collection_items = limits.collection_items.min(64);
    limits.total_entries = limits.total_entries.min(256);
    crate::preflight::preflight_qualified(input, ".nearfuture.ipc.v1.LocalAuthEnvelope", limits)?;
    let value = a::LocalAuthEnvelope::decode(input).map_err(|_| WireError::Malformed)?;
    if value.auth_version != 1 || value.capability_id != 2 || value.schema_id != 2 {
        return Err(WireError::Unsupported);
    }
    if value
        .runtime_session
        .as_ref()
        .ok_or(WireError::Semantic)?
        .value
        == 0
    {
        return Err(WireError::Semantic);
    }
    match value.body.as_ref().ok_or(WireError::Semantic)? {
        a::local_auth_envelope::Body::Hello(h) => {
            if h.client_nonce.len() != 32 || !(1..=2).contains(&h.endpoint_role) {
                return Err(WireError::Semantic);
            }
            let p = h.principal.as_ref().ok_or(WireError::Semantic)?;
            if p.account_id
                .as_ref()
                .ok_or(WireError::Semantic)?
                .value
                .len()
                != 16
                || p.device_id.as_ref().ok_or(WireError::Semantic)?.value.len() != 16
            {
                return Err(WireError::Semantic);
            }
            let protocols = h.protocols.as_ref().ok_or(WireError::Semantic)?;
            if protocols.minimum != 1 || protocols.maximum != 1 {
                return Err(WireError::Unsupported);
            }
            crate::admission::limits(h.offered_limits.as_ref().ok_or(WireError::Semantic)?)?;
        }
        a::local_auth_envelope::Body::Challenge(h) => {
            if h.server_nonce.len() != 32 || h.server_proof.len() != 32 {
                return Err(WireError::Semantic);
            }
            crate::admission::limits(h.selected_limits.as_ref().ok_or(WireError::Semantic)?)?;
        }
        a::local_auth_envelope::Body::Proof(h) => {
            if h.client_proof.len() != 32 {
                return Err(WireError::Semantic);
            }
        }
        a::local_auth_envelope::Body::Accepted(h) => {
            if h.server_finished_proof.len() != 32 {
                return Err(WireError::Semantic);
            }
        }
    }
    Ok(value)
}
