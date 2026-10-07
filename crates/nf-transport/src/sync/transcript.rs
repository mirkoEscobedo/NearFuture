use super::{
    fields::{put_limits, put_peer},
    reader::Writer,
    *,
};
use crate::{PeerError, records::PeerContext};
use libp2p::PeerId;
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncAuthStage {
    Neutral,
    Server,
    Client,
    Finished,
}
/// Explicit transcript values only; construction does not admit principals or a policy mapping.
#[derive(Clone, Debug)]
pub struct SyncAuthTranscript {
    pub lane: SyncLane,
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
    pub available: u32,
    pub selected_caps: u32,
    pub offered: SyncLimits,
    pub server_limits: SyncLimits,
    pub selected: SyncLimits,
    pub membership: MemberStamp,
    pub pins: ExpectedProfilePins,
}
impl SyncAuthTranscript {
    pub fn encode(
        &self,
        stage: SyncAuthStage,
        expected: ExpectedProfilePins,
    ) -> Result<[u8; 750], PeerError> {
        self.validate(expected)?;
        let mut w = Writer {
            bytes: Vec::with_capacity(750),
        };
        w.raw(b"NF-SYNC-AUTH-1\0");
        w.u8(match stage {
            SyncAuthStage::Neutral => 0,
            SyncAuthStage::Server => 1,
            SyncAuthStage::Client => 2,
            SyncAuthStage::Finished => 3,
        });
        w.u16(1);
        w.u8(match self.lane {
            SyncLane::Control => 1,
            SyncLane::Transfer => 2,
        });
        put_peer(&mut w, self.client_peer);
        put_peer(&mut w, self.server_peer);
        for v in [
            self.client_account.as_bytes(),
            self.client_device.as_bytes(),
            self.server_account.as_bytes(),
            self.server_device.as_bytes(),
        ] {
            w.raw(v);
        }
        w.raw(&self.context.session);
        w.raw(self.context.scope.universe.as_bytes());
        w.raw(self.context.scope.history.as_bytes());
        w.raw(&self.context.ruleset);
        w.raw(&self.context.content);
        w.raw(&self.client_nonce);
        w.raw(&self.server_nonce);
        for n in [
            self.required,
            self.optional,
            self.available,
            self.selected_caps,
        ] {
            w.u32(n);
        }
        for v in [self.offered, self.server_limits, self.selected] {
            put_limits(&mut w, v);
        }
        w.u64(if stage == SyncAuthStage::Neutral {
            0
        } else {
            self.membership.revision
        });
        w.u16(1);
        w.raw(&self.pins.implementation);
        w.raw(&self.pins.schema);
        w.raw(&self.membership.digest);
        w.raw(&Sha256::digest(self.lane.protocol().as_bytes()));
        w.u8(1);
        w.bytes.try_into().map_err(|_| PeerError::Malformed)
    }
    fn validate(&self, expected: ExpectedProfilePins) -> Result<(), PeerError> {
        if self.pins != expected
            || self.required != 1
            || self.optional != 0
            || self.available != 1
            || self.selected_caps != 1
        {
            return Err(PeerError::Unsupported);
        }
        if self.selected != self.offered.negotiate(self.server_limits)? {
            return Err(PeerError::Limit);
        }
        if self.context.session == [0; 16] {
            return Err(PeerError::Session);
        }
        for v in [
            self.client_account.as_bytes(),
            self.client_device.as_bytes(),
            self.server_account.as_bytes(),
            self.server_device.as_bytes(),
            self.context.scope.universe.as_bytes(),
            self.context.scope.history.as_bytes(),
        ] {
            if *v == [0; 16] {
                return Err(PeerError::Malformed);
            }
        }
        if self.client_peer.to_bytes().len() > 128 || self.server_peer.to_bytes().len() > 128 {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn digest(
        &self,
        stage: SyncAuthStage,
        expected: ExpectedProfilePins,
    ) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.encode(stage, expected)?).into())
    }
}
