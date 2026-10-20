use super::{bytes::hash, stamp::OwnerStamp};
use nf_contract::identity::RequestId;
use nf_identity::model::DeviceProof;

/// Immutable signing input issued by an actual private coordinator book.
pub struct Challenge {
    pub(super) transcript: Vec<u8>,
    pub(super) admission: Vec<u8>,
    pub(super) unsigned: Vec<u8>,
    pub(super) delta: Vec<u8>,
    pub(super) book: [u8; 32],
}
impl Challenge {
    pub fn transcript(&self) -> &[u8] {
        &self.transcript
    }
    pub fn digest(&self) -> [u8; 32] {
        hash(&self.transcript)
    }
    pub fn admission_input_bytes(&self) -> &[u8] {
        &self.admission
    }
    pub fn unsigned_payload(&self) -> &[u8] {
        &self.unsigned
    }
    pub fn registration_delta_bytes(&self) -> &[u8] {
        &self.delta
    }
    pub fn bind_proof(self, proof: DeviceProof) -> PrincipalProof {
        PrincipalProof {
            book: self.book,
            transcript: self.transcript,
            proof,
        }
    }
}
/// A supplied signature still requires consumption and verification by its issuer.
pub struct PrincipalProof {
    pub(super) book: [u8; 32],
    pub(super) transcript: Vec<u8>,
    pub(super) proof: DeviceProof,
}
#[derive(Clone)]
pub(super) struct Times {
    pub auth_sequence: u64,
    pub nonce: [u8; 32],
    pub issued: u64,
    pub expires: u64,
}
#[derive(Clone)]
pub(super) struct Window {
    pub nonce: [u8; 32],
    pub issued: u64,
    pub expires: u64,
}
impl Times {
    pub fn window(&self) -> Window {
        Window {
            nonce: self.nonce,
            issued: self.issued,
            expires: self.expires,
        }
    }
}
#[derive(Clone)]
pub(super) struct Owned {
    pub stamp: OwnerStamp,
    pub purpose: u32,
    pub sequence: u64,
    pub operation: [u8; 16],
    pub revision: u64,
    pub state: [u8; 32],
    pub head: [u8; 32],
    pub payload: [u8; 32],
    pub times: Times,
}
#[derive(Clone)]
pub(super) struct Principal {
    pub stamp: OwnerStamp,
    pub sequence: u64,
    pub operation: [u8; 16],
    pub revision: u64,
    pub state: [u8; 32],
    pub head: [u8; 32],
    pub frontier: [u8; 32],
    pub request: RequestId,
    pub actor: nf_contract::identity::AccountId,
    pub device: nf_contract::identity::DeviceId,
    pub intent: [u8; 32],
    pub times: Window,
}
#[derive(Clone)]
pub(super) enum Binding {
    Owned(Owned),
    Prospective {
        purpose: u32,
        candidate: OwnerStamp,
        input: [u8; 32],
    },
    Principal(Principal),
}
