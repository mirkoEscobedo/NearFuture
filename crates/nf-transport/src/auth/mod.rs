mod bulk;
pub use bulk::BulkTranscript;
mod query;
pub use query::QueryTranscript;
mod client;
use crate::{
    PeerError,
    records::{Lane, PeerContext, PeerLimits},
};
pub use client::{ClientHandshake, ServerPin};
use libp2p::PeerId;
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
#[derive(Clone, Debug)]
pub struct HandshakeContext {
    pub lane: Lane,
    pub client_peer: PeerId,
    pub server_peer: PeerId,
    pub client_account: AccountId,
    pub client_device: DeviceId,
    pub server_account: AccountId,
    pub server_device: DeviceId,
    pub context: PeerContext,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub required: u32,
    pub optional: u32,
    pub server_available: u32,
    pub selected_caps: u32,
    pub offered: PeerLimits,
    pub server_limits: PeerLimits,
    pub selected: PeerLimits,
}
impl HandshakeContext {
    pub fn validate(&self) -> Result<(), PeerError> {
        if self.context.session == [0; 16] {
            return Err(PeerError::Session);
        }
        if self.required & !3 != 0
            || self.required & self.lane as u32 == 0
            || self.server_available & !3 != 0
            || self.required & self.server_available != self.required
            || self.selected_caps != (self.required | self.optional) & self.server_available
        {
            return Err(PeerError::Unsupported);
        }
        if self.offered.negotiate(self.server_limits)? != self.selected {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn challenge(&self, stage: u8, frontier: u64) -> Result<[u8; 32], PeerError> {
        if !(1..=3).contains(&stage) {
            return Err(PeerError::Malformed);
        }
        self.digest(stage, frontier)
    }
    pub fn context_digest(&self) -> Result<[u8; 32], PeerError> {
        self.digest(0, 0)
    }
    fn digest(&self, stage: u8, frontier: u64) -> Result<[u8; 32], PeerError> {
        self.validate()?;
        let mut b = Vec::with_capacity(619);
        b.extend(b"NF-PEER-AUTH-1\0");
        b.push(stage);
        b.extend(1u16.to_le_bytes());
        b.push(self.lane as u8);
        for peer in [self.client_peer, self.server_peer] {
            crate::records::fields::write_peer(&mut b, &peer.to_bytes())?;
        }
        b.extend(self.client_account.as_bytes());
        b.extend(self.client_device.as_bytes());
        b.extend(self.server_account.as_bytes());
        b.extend(self.server_device.as_bytes());
        b.extend(self.context.session);
        b.extend(self.context.scope.universe.as_bytes());
        b.extend(self.context.scope.history.as_bytes());
        b.extend(self.context.ruleset);
        b.extend(self.context.content);
        b.extend(self.client_nonce);
        b.extend(self.server_nonce);
        for v in [
            self.required,
            self.optional,
            self.server_available,
            self.selected_caps,
        ] {
            b.extend(v.to_le_bytes());
        }
        for l in [self.offered, self.server_limits, self.selected] {
            crate::records::fields::write_limits(&mut b, l);
        }
        b.extend(frontier.to_le_bytes());
        if b.len() != 619 {
            return Err(PeerError::Malformed);
        }
        Ok(Sha256::digest(b).into())
    }
}
