//! Generated protobuf contracts and strict data-only admission. No transport or mutation.
pub mod nearfuture {
    pub mod protocol {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/nearfuture.protocol.v1.rs"));
        }
    }
    pub mod ipc {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/nearfuture.ipc.v1.rs"));
        }
    }
}
pub use nearfuture::ipc::v1 as local_auth;
pub use nearfuture::protocol::v1 as generated;
mod admission;
mod local_auth_admission;
pub use local_auth_admission::{decode_local_auth, decode_local_auth_with_limits};
mod preflight;
mod records;
use prost::Message;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    Malformed,
    Duplicate,
    UnknownField,
    Limit,
    Unsupported,
    Semantic,
}
/// Explicit hard ceilings; callers may lower but never raise these budgets.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub frame_bytes: usize,
    pub field_bytes: usize,
    pub text_bytes: usize,
    pub depth: usize,
    pub collection_items: usize,
    pub total_entries: usize,
    pub decoded_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            frame_bytes: 1_048_576,
            field_bytes: 262_144,
            text_bytes: 4096,
            depth: 32,
            collection_items: 4096,
            total_entries: 16384,
            decoded_bytes: 1_048_576,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), WireError> {
        let hard = Self::default();
        if self.frame_bytes == 0
            || self.frame_bytes > hard.frame_bytes
            || self.field_bytes == 0
            || self.field_bytes > hard.field_bytes
            || self.text_bytes == 0
            || self.text_bytes > hard.text_bytes
            || self.depth == 0
            || self.depth > hard.depth
            || self.collection_items == 0
            || self.collection_items > hard.collection_items
            || self.total_entries == 0
            || self.total_entries > hard.total_entries
            || self.decoded_bytes == 0
            || self.decoded_bytes > hard.decoded_bytes
        {
            return Err(WireError::Limit);
        }
        Ok(())
    }
}
pub fn decode_control(input: &[u8]) -> Result<generated::ControlEnvelope, WireError> {
    decode_control_with_limits(input, Limits::default())
}
pub fn decode_control_with_limits(
    input: &[u8],
    limits: Limits,
) -> Result<generated::ControlEnvelope, WireError> {
    preflight::preflight(input, "ControlEnvelope", limits)?;
    let value = generated::ControlEnvelope::decode(input).map_err(|_| WireError::Malformed)?;
    admission::control(&value)?;
    Ok(value)
}
/// One exact u32 big-endian frame. This function performs no I/O.
pub fn decode_frame(input: &[u8]) -> Result<generated::ControlEnvelope, WireError> {
    let header = input.get(..4).ok_or(WireError::Malformed)?;
    let length = u32::from_be_bytes(header.try_into().map_err(|_| WireError::Malformed)?) as usize;
    if length > Limits::default().frame_bytes {
        return Err(WireError::Limit);
    }
    if length == 0 || input.len() != length + 4 {
        return Err(WireError::Malformed);
    }
    decode_control(&input[4..])
}
pub fn decode_snapshot(input: &[u8]) -> Result<generated::WorldSnapshot, WireError> {
    preflight::preflight(input, "WorldSnapshot", Limits::default())?;
    let value = generated::WorldSnapshot::decode(input).map_err(|_| WireError::Malformed)?;
    admission::snapshot(&value)?;
    Ok(value)
}
pub fn decode_chunk(input: &[u8]) -> Result<generated::SnapshotChunk, WireError> {
    decode_chunk_with_limits(input, Limits::default())
}
/// Borrowed preflight enforces negotiated quotas before generated payload allocation.
pub fn decode_chunk_with_limits(
    input: &[u8],
    limits: Limits,
) -> Result<generated::SnapshotChunk, WireError> {
    preflight::preflight(input, "SnapshotChunk", limits)?;
    let value = generated::SnapshotChunk::decode(input).map_err(|_| WireError::Malformed)?;
    admission::chunk(&value)?;
    Ok(value)
}
impl WireError {
    /// Safe static reasons only. No incoming values, paths, tokens or stack traces.
    pub fn bounded_error(self) -> generated::BoundedError {
        let (code, reason) = match self {
            Self::Malformed | Self::Duplicate | Self::UnknownField => {
                (generated::ErrorCode::Malformed, "Malformed protocol input.")
            }
            Self::Limit => (
                generated::ErrorCode::LimitExceeded,
                "Protocol resource limit exceeded.",
            ),
            Self::Unsupported => (
                generated::ErrorCode::UnknownRequiredSemantic,
                "Unsupported required protocol, capability or schema.",
            ),
            Self::Semantic => (
                generated::ErrorCode::Malformed,
                "Invalid protocol semantics.",
            ),
        };
        generated::BoundedError {
            code: code as i32,
            reason: reason.into(),
            unsupported_capability_ids: vec![],
            unsupported_schema_ids: vec![],
            retryable: false,
        }
    }
}
