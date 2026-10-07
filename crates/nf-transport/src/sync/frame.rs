use super::*;
use crate::PeerError;
/// Bounded complete-frame data helper; no socket, timeout or stream ownership.
pub fn decode_frame(input: &[u8], policy: SyncWirePolicy) -> Result<SyncRecord, PeerError> {
    policy.validate()?;
    let prefix: [u8; 4] = input
        .get(..4)
        .ok_or(PeerError::Malformed)?
        .try_into()
        .map_err(|_| PeerError::Malformed)?;
    let n = u32::from_be_bytes(prefix) as usize;
    if !(HEADER_BYTES..=policy.frame_maximum()?).contains(&n) {
        return Err(PeerError::Limit);
    }
    if input.len() != n.checked_add(4).ok_or(PeerError::Limit)? {
        return Err(PeerError::Malformed);
    }
    decode_body(&input[4..], policy)
}
pub fn encode_frame(record: &SyncRecord, policy: SyncWirePolicy) -> Result<Vec<u8>, PeerError> {
    let body = encode_body(record, policy)?;
    let mut frame = Vec::with_capacity(body.len() + 4);
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}
