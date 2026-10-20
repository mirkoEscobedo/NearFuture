//! Bounded operator input. Parsing creates data, never opens state or grants authority.
use crate::{
    PeerError,
    auth::ServerPin,
    receipt::{OriginalReceipt, SourceMinima},
};
use libp2p::Multiaddr;
use nf_contract::identity::*;
use nf_identity::model::Scope;
use nf_store::KnownFrontiers;
mod arguments;
mod file;
mod parser;
mod primitives;
mod rows;
pub use arguments::PortalArguments;
pub use file::read_config;
pub const MAX_CONFIG_BYTES: usize = 16_384;
pub const MAX_PATH_BYTES: usize = 4096;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortalMode {
    Serve,
    Watch,
    InitializeBook,
}
#[derive(Clone, Debug)]
pub struct OriginalConfig {
    pub slot: u8,
    pub original: OriginalReceipt,
    pub minimum: SourceMinima,
    pub head: Option<(u8, [u8; 32])>,
}
#[derive(Clone, Debug)]
pub struct PortalConfig {
    pub mode: PortalMode,
    pub local: KnownFrontiers,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub local_account: AccountId,
    pub local_device: DeviceId,
    pub server: ServerPin,
    pub addresses: Option<[Multiaddr; 3]>,
    pub originals: Vec<OriginalConfig>,
}
impl PortalConfig {
    pub fn parse(bytes: &[u8], expected: PortalMode) -> Result<Self, PeerError> {
        parser::parse(bytes, expected)
    }
    pub fn scope(&self) -> Scope {
        self.local.scope
    }
    pub fn protected_membership(&self) -> Result<u64, PeerError> {
        Ok(self
            .local
            .membership_revision
            .ok_or(PeerError::Policy)?
            .max(self.server.minimum_membership))
    }
    pub fn selected_original(&self, slot: u8) -> Result<&OriginalConfig, PeerError> {
        self.originals
            .iter()
            .find(|row| row.slot == slot)
            .ok_or(PeerError::Malformed)
    }
}
