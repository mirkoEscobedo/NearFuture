use super::NotifyLimits;
use crate::PeerError;
/// Pure length admission for a future framed reader; performs no I/O or allocation.
pub fn admit_frame_length(prefix: [u8; 4], limits: NotifyLimits) -> Result<usize, PeerError> {
    limits.validate()?;
    let length = usize::try_from(u32::from_be_bytes(prefix)).map_err(|_| PeerError::Limit)?;
    if length > usize::from(limits.frame) {
        return Err(PeerError::Limit);
    }
    if length < 128 {
        return Err(PeerError::Malformed);
    }
    Ok(length)
}
