//! Closed notification wire values. Shape admission is never application authorization.
mod codec;
mod decode;
mod fields;
mod limits;
mod model;
mod validation;
pub use codec::encode_body;
pub use decode::decode_body;
pub use model::{NotifyBody, NotifyContext, NotifyLimits, NotifyRecord, NotifySelector, PROTOCOL};
mod framing;
pub use framing::admit_frame_length;
mod transcript;
pub use transcript::signed_prefix_digest;
pub use transcript::{
    NotifyHandshakeTranscript, NotifyNoticeTranscript, NotifySubscribeTranscript, signed_prefix,
};
