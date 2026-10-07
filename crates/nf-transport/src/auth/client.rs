use super::HandshakeContext;
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::model::{DeviceProof, MembershipState, ProtectedOperation};
#[derive(Clone, Copy, Debug)]
pub struct ServerPin {
    pub peer: libp2p::PeerId,
    pub account: AccountId,
    pub device: DeviceId,
    pub minimum_membership: u64,
}
/// The trusted owner must load current durable membership for each call. No membership is cached here.
pub struct ClientHandshake {
    context: HandshakeContext,
    pin: ServerPin,
    next: u8,
    hello_revision: Option<u64>,
}
impl ClientHandshake {
    pub fn new(context: HandshakeContext, pin: ServerPin) -> Result<Self, PeerError> {
        context.validate()?;
        if pin.peer != context.server_peer
            || pin.account != context.server_account
            || pin.device != context.server_device
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(Self {
            context,
            pin,
            next: 1,
            hello_revision: None,
        })
    }
    pub(crate) fn revision(&self) -> Option<u64> {
        self.hello_revision
    }
    pub fn active(&self) -> bool {
        self.next == 4
    }
    pub fn invalidate(&mut self) {
        self.next = 0;
        self.hello_revision = None;
    }
    pub fn accept_server_hello(
        &mut self,
        proof: &DeviceProof,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        self.accept(1, proof, current)?;
        self.hello_revision = Some(current.revision);
        self.next = 3;
        Ok(())
    }
    pub fn accept_finished(
        &mut self,
        proof: &DeviceProof,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        self.accept(3, proof, current)?;
        self.next = 4;
        Ok(())
    }
    fn accept(
        &mut self,
        stage: u8,
        proof: &DeviceProof,
        current: &MembershipState,
    ) -> Result<(), PeerError> {
        let expected = self.next;
        self.next = 0;
        if expected != stage {
            return Err(PeerError::Replay);
        }
        if current.owner != self.pin.account
            || current.scope != self.context.context.scope
            || proof.account != self.pin.account
            || proof.device != self.pin.device
            || self.hello_revision.is_some_and(|v| v != current.revision)
        {
            return Err(PeerError::Unauthorized);
        }
        let challenge = self.context.challenge(stage, proof.frontier)?;
        current
            .authorize(
                proof,
                &self.pin.peer.to_bytes(),
                &challenge,
                self.pin.minimum_membership,
                ProtectedOperation::Economic,
            )
            .map_err(|_| PeerError::Unauthorized)?;
        Ok(())
    }
}
