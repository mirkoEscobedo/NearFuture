//! Canonical peer syntax only; no role, key, connection or authority grant.
use crate::model::IdentityError;
use libp2p_identity::PeerId;

/// Validates a nonempty bounded peer with the maintained parser and exact encoding.
/// This preserves its accepted multihash profile; it does not validate key possession.
pub fn validate_canonical_peer(bytes: &[u8]) -> Result<(), IdentityError> {
    if bytes.is_empty() || bytes.len() > 128 {
        return Err(IdentityError::Limit);
    }
    let peer = PeerId::from_bytes(bytes).map_err(|_| IdentityError::Malformed)?;
    let mut scratch = [0u8; 128];
    let written = peer
        .as_ref()
        .write(&mut scratch[..])
        .map_err(|_| IdentityError::Malformed)?;
    if written != bytes.len() || scratch[..written] != *bytes {
        return Err(IdentityError::Malformed);
    }
    Ok(())
}
