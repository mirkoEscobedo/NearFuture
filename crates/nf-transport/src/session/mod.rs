mod binding;
pub(crate) use binding::{Binding, sign_challenge};
mod client;
mod server;
use crate::{
    PeerError,
    auth::HandshakeContext,
    records::{Lane, PeerContext, PeerLimits, PeerRecord, encode_body},
};
pub use client::ClientSession;
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipState, ProtectedOperation, PublicIdentity, Scope},
    signing::device_digest,
};
pub use server::ServerSession;
#[derive(Clone, Copy, Debug)]
pub struct SessionPolicy {
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub limits: PeerLimits,
    pub minimum_membership: u64,
}
impl SessionPolicy {
    pub(crate) fn context(self, session: [u8; 16]) -> PeerContext {
        PeerContext {
            session,
            scope: self.scope,
            ruleset: self.ruleset,
            content: self.content,
        }
    }
    pub(crate) fn admit(self, record: &PeerRecord, lane: Lane) -> Result<(), PeerError> {
        encode_body(record, lane, self.limits)?;
        if record.context.scope != self.scope {
            return Err(PeerError::Scope);
        }
        if record.context.ruleset != self.ruleset || record.context.content != self.content {
            return Err(PeerError::Policy);
        }
        Ok(())
    }
}
pub(crate) fn fresh<const N: usize>() -> Result<[u8; N], PeerError> {
    let mut out = [0; N];
    getrandom::fill(&mut out).map_err(|_| PeerError::Entropy)?;
    if out == [0; N] {
        return Err(PeerError::Entropy);
    }
    Ok(out)
}
pub(crate) fn local_matches(local: &PublicIdentity, peer: libp2p::PeerId) -> Result<(), PeerError> {
    if local.peer != peer.to_bytes() {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
pub(crate) fn sign_stage(
    c: &HandshakeContext,
    stage: u8,
    current: &MembershipState,
    local: &PublicIdentity,
    key: &SecretSeed,
    minimum: u64,
) -> Result<DeviceProof, PeerError> {
    if current.scope != c.context.scope || key.public_key() != local.device_key {
        return Err(PeerError::Unauthorized);
    }
    let challenge = c.challenge(stage, current.revision)?;
    let mut proof = DeviceProof {
        scope: current.scope,
        account: local.account,
        device: local.device,
        frontier: current.revision,
        peer: local.peer.clone(),
        challenge,
        signature: [0; 64],
    };
    proof.signature = key.sign(&device_digest(&proof).map_err(|_| PeerError::Unauthorized)?);
    current
        .authorize(
            &proof,
            &local.peer,
            &challenge,
            minimum,
            ProtectedOperation::Economic,
        )
        .map_err(|_| PeerError::Unauthorized)?;
    Ok(proof)
}
