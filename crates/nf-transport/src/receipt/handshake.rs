use crate::{
    PeerError,
    auth::HandshakeContext,
    records::{
        Lane,
        fields::{write_limits, write_peer},
    },
};
use sha2::{Digest, Sha256};
/// Control2 transcript data. This performs no membership lookup or authorization.
#[derive(Clone, Debug)]
pub struct ReceiptHandshake {
    context: HandshakeContext,
}
impl ReceiptHandshake {
    pub fn new(context: HandshakeContext) -> Result<Self, PeerError> {
        context.validate()?;
        if context.lane != Lane::Control
            || context.required != 1
            || context.server_available != 1
            || context.selected_caps != 1
        {
            return Err(PeerError::Unsupported);
        }
        if context.client_nonce == [0; 32]
            || context.server_nonce == [0; 32]
            || [
                context.client_account.as_bytes(),
                context.client_device.as_bytes(),
                context.server_account.as_bytes(),
                context.server_device.as_bytes(),
            ]
            .iter()
            .any(|v| **v == [0; 16])
        {
            return Err(PeerError::Malformed);
        }
        Ok(Self { context })
    }
    pub fn context(&self) -> &HandshakeContext {
        &self.context
    }
    pub fn transcript(&self, stage: u8, frontier: u64) -> Result<[u8; 619], PeerError> {
        if stage > 3 || (stage == 0 && frontier != 0) {
            return Err(PeerError::Malformed);
        }
        let c = &self.context;
        let mut b = Vec::with_capacity(619);
        b.extend(b"NF-PEER-AUTH-2\0");
        b.push(stage);
        b.extend(2u16.to_le_bytes());
        b.push(1);
        for p in [c.client_peer, c.server_peer] {
            write_peer(&mut b, &p.to_bytes())?;
        }
        for id in [
            c.client_account.as_bytes(),
            c.client_device.as_bytes(),
            c.server_account.as_bytes(),
            c.server_device.as_bytes(),
        ] {
            b.extend(id);
        }
        b.extend(c.context.session);
        b.extend(c.context.scope.universe.as_bytes());
        b.extend(c.context.scope.history.as_bytes());
        b.extend(c.context.ruleset);
        b.extend(c.context.content);
        b.extend(c.client_nonce);
        b.extend(c.server_nonce);
        for v in [c.required, c.optional, c.server_available, c.selected_caps] {
            b.extend(v.to_le_bytes());
        }
        for l in [c.offered, c.server_limits, c.selected] {
            write_limits(&mut b, l);
        }
        b.extend(frontier.to_le_bytes());
        b.try_into().map_err(|_| PeerError::Malformed)
    }
    pub fn context_digest(&self) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(self.transcript(0, 0)?).into())
    }
    pub fn challenge(&self, stage: u8, frontier: u64) -> Result<[u8; 32], PeerError> {
        if !(1..=3).contains(&stage) {
            return Err(PeerError::Malformed);
        }
        Ok(Sha256::digest(self.transcript(stage, frontier)?).into())
    }
}
