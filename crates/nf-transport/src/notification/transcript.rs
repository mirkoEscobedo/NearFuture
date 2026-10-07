use super::{NotifyContext, NotifyLimits, NotifyRecord, NotifySelector};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
#[derive(Clone, Debug)]
pub struct NotifyHandshakeTranscript {
    pub client_peer: libp2p::PeerId,
    pub server_peer: libp2p::PeerId,
    pub client_account: AccountId,
    pub client_device: DeviceId,
    pub server_account: AccountId,
    pub server_device: DeviceId,
    pub context: NotifyContext,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub required: u32,
    pub optional: u32,
    pub server_available: u32,
    pub selected_caps: u32,
    pub offered: NotifyLimits,
    pub server_limits: NotifyLimits,
    pub selected: NotifyLimits,
}
impl NotifyHandshakeTranscript {
    pub fn preimage(
        &self,
        stage: u8,
        frontier: u64,
        actual_protocol: &str,
    ) -> Result<Vec<u8>, PeerError> {
        if actual_protocol != super::PROTOCOL {
            return Err(PeerError::Unsupported);
        }
        if stage > 3 || stage == 0 && frontier != 0 {
            return Err(PeerError::Malformed);
        }
        super::validation::context(self.context, false)?;
        for id in [
            self.client_account.as_bytes(),
            self.client_device.as_bytes(),
            self.server_account.as_bytes(),
            self.server_device.as_bytes(),
        ] {
            super::validation::nonzero(id)?;
        }
        super::validation::nonzero(&self.client_nonce)?;
        super::validation::nonzero(&self.server_nonce)?;
        if self.required != 1
            || self.optional != 0
            || self.server_available != 1
            || self.selected_caps != 1
        {
            return Err(PeerError::Unsupported);
        }
        if self.offered.negotiate(self.server_limits)? != self.selected {
            return Err(PeerError::Limit);
        }
        let mut b = Vec::with_capacity(617);
        b.extend(b"NF-NOTIFY-AUTH-1\0");
        b.push(stage);
        b.extend(1u16.to_le_bytes());
        b.push(3);
        b.extend(Sha256::digest(super::PROTOCOL.as_bytes()));
        for peer in [self.client_peer, self.server_peer] {
            crate::records::fields::write_peer(&mut b, &peer.to_bytes())?;
        }
        for id in [
            self.client_account.as_bytes(),
            self.client_device.as_bytes(),
            self.server_account.as_bytes(),
            self.server_device.as_bytes(),
        ] {
            b.extend(id);
        }
        b.extend(self.context.session);
        b.extend(self.context.scope.universe.as_bytes());
        b.extend(self.context.scope.history.as_bytes());
        b.extend(self.context.ruleset);
        b.extend(self.context.content);
        b.extend(self.client_nonce);
        b.extend(self.server_nonce);
        for n in [
            self.required,
            self.optional,
            self.server_available,
            self.selected_caps,
        ] {
            b.extend(n.to_le_bytes());
        }
        for limits in [self.offered, self.server_limits, self.selected] {
            super::fields::write_limits(&mut b, limits);
        }
        b.extend(frontier.to_le_bytes());
        Ok(b)
    }
}
#[derive(Clone, Debug)]
pub struct NotifySubscribeTranscript {
    pub context_digest: [u8; 32],
    pub subscription: [u8; 16],
    pub selector: NotifySelector,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub frontier: u64,
    pub minimum_membership: u64,
    pub lifetime: u16,
}
impl NotifySubscribeTranscript {
    pub fn preimage(&self, stage: u8, reply_digest: [u8; 32]) -> Result<Vec<u8>, PeerError> {
        if !(1..=2).contains(&stage) || stage == 1 && reply_digest != [0; 32] {
            return Err(PeerError::Malformed);
        }
        super::validation::nonzero(&self.subscription)?;
        super::validation::selector(&self.selector)?;
        super::validation::nonzero(&self.client_nonce)?;
        super::validation::nonzero(&self.server_nonce)?;
        if self.frontier < self.minimum_membership {
            return Err(PeerError::Unauthorized);
        }
        if !(1..=30).contains(&self.lifetime) {
            return Err(PeerError::Limit);
        }
        let mut b = Vec::with_capacity(244);
        b.extend(b"NF-NOTIFY-SUB-1\0");
        b.push(stage);
        b.extend(self.context_digest);
        b.extend(self.subscription);
        b.push(1);
        super::fields::write_selector(&mut b, &self.selector);
        b.extend(self.client_nonce);
        b.extend(self.server_nonce);
        b.extend(self.frontier.to_le_bytes());
        b.extend(self.minimum_membership.to_le_bytes());
        b.extend(self.lifetime.to_le_bytes());
        b.extend(reply_digest);
        Ok(b)
    }
}
#[derive(Clone, Debug)]
pub struct NotifyNoticeTranscript {
    pub context_digest: [u8; 32],
    pub subscription: [u8; 16],
    pub sequence: u64,
    pub selector: NotifySelector,
    pub nonce: [u8; 32],
    pub frontier: u64,
}
impl NotifyNoticeTranscript {
    pub fn preimage(&self, stage: u8, prefix_digest: [u8; 32]) -> Result<Vec<u8>, PeerError> {
        if !(1..=2).contains(&stage) || self.sequence == 0 {
            return Err(PeerError::Malformed);
        }
        super::validation::nonzero(&self.subscription)?;
        super::validation::selector(&self.selector)?;
        super::validation::nonzero(&self.nonce)?;
        let mut b = Vec::with_capacity(212);
        b.extend(b"NF-NOTIFY-NOTICE-1\0");
        b.push(stage);
        b.extend(self.context_digest);
        b.extend(self.subscription);
        b.extend(self.sequence.to_le_bytes());
        super::fields::write_selector(&mut b, &self.selector);
        b.extend(self.nonce);
        b.extend(self.frontier.to_le_bytes());
        b.extend(prefix_digest);
        Ok(b)
    }
}
pub fn signed_prefix(
    record: &NotifyRecord,
    actual_protocol: &str,
    limits: NotifyLimits,
) -> Result<Vec<u8>, PeerError> {
    if !matches!(
        record.body,
        super::NotifyBody::Subscribed { .. }
            | super::NotifyBody::Notice { .. }
            | super::NotifyBody::NoticeAck { .. }
    ) {
        return Err(PeerError::Unsupported);
    }
    let mut bytes = super::encode_body(record, actual_protocol, limits)?;
    let end = bytes.len().checked_sub(297).ok_or(PeerError::Malformed)?;
    bytes.truncate(end);
    Ok(bytes)
}
impl NotifyHandshakeTranscript {
    pub fn context_digest(&self, actual_protocol: &str) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.preimage(0, 0, actual_protocol)?).into())
    }
    pub fn challenge(
        &self,
        stage: u8,
        frontier: u64,
        actual_protocol: &str,
    ) -> Result<[u8; 32], PeerError> {
        if !(1..=3).contains(&stage) {
            return Err(PeerError::Malformed);
        }
        Ok(Sha256::digest(self.preimage(stage, frontier, actual_protocol)?).into())
    }
}
impl NotifySubscribeTranscript {
    pub fn challenge(&self, stage: u8, prefix: [u8; 32]) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.preimage(stage, prefix)?).into())
    }
}
impl NotifyNoticeTranscript {
    pub fn challenge(&self, stage: u8, prefix: [u8; 32]) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.preimage(stage, prefix)?).into())
    }
}
pub fn signed_prefix_digest(
    record: &NotifyRecord,
    actual_protocol: &str,
    limits: NotifyLimits,
) -> Result<[u8; 32], PeerError> {
    Ok(Sha256::digest(signed_prefix(record, actual_protocol, limits)?).into())
}
