//! Bounded canonical Chat value transport; Noise and decoded shapes grant no application authority.
pub mod codec;
pub mod framing;
mod model;
pub use model::{ChatFrame, MAX_FRAME_BYTES, PROTOCOL, Refusal, WireContext};
mod admission;
mod client;
pub mod network;
mod owner;
pub use client::deliver_pending;
pub use owner::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent};

pub mod history;
mod history_client;
pub use history_client::fetch_history;
