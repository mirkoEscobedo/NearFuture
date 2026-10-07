use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, RequestId};
use nf_identity::model::Scope;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lane {
    Control = 1,
    Bulk = 2,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerLimits {
    pub control_frame: u32,
    pub bulk_frame: u32,
    pub chunk_bytes: u32,
    pub control_queue_bytes: u32,
    pub bulk_queue_bytes: u32,
    pub control_items: u16,
    pub bulk_items: u16,
    pub pending_challenges: u16,
}
impl Default for PeerLimits {
    fn default() -> Self {
        Self {
            control_frame: 4096,
            bulk_frame: 9216,
            chunk_bytes: 8192,
            control_queue_bytes: 65536,
            bulk_queue_bytes: 131072,
            control_items: 16,
            bulk_items: 4,
            pending_challenges: 8,
        }
    }
}
impl PeerLimits {
    pub fn validate(self) -> Result<(), PeerError> {
        let hard = Self::default();
        let actual = [
            self.control_frame,
            self.bulk_frame,
            self.chunk_bytes,
            self.control_queue_bytes,
            self.bulk_queue_bytes,
            self.control_items as u32,
            self.bulk_items as u32,
            self.pending_challenges as u32,
        ];
        let ceiling = [
            hard.control_frame,
            hard.bulk_frame,
            hard.chunk_bytes,
            hard.control_queue_bytes,
            hard.bulk_queue_bytes,
            hard.control_items as u32,
            hard.bulk_items as u32,
            hard.pending_challenges as u32,
        ];
        if actual.iter().zip(ceiling).any(|(a, c)| *a == 0 || *a > c)
            || self.control_frame < 1024
            || self.bulk_frame < 1024
            || self.chunk_bytes > self.bulk_frame - 146
            || self.control_queue_bytes < self.control_frame
            || self.bulk_queue_bytes < self.bulk_frame
        {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn negotiate(self, other: Self) -> Result<Self, PeerError> {
        self.validate()?;
        other.validate()?;
        let value = Self {
            control_frame: self.control_frame.min(other.control_frame),
            bulk_frame: self.bulk_frame.min(other.bulk_frame),
            chunk_bytes: self.chunk_bytes.min(other.chunk_bytes),
            control_queue_bytes: self.control_queue_bytes.min(other.control_queue_bytes),
            bulk_queue_bytes: self.bulk_queue_bytes.min(other.bulk_queue_bytes),
            control_items: self.control_items.min(other.control_items),
            bulk_items: self.bulk_items.min(other.bulk_items),
            pending_challenges: self.pending_challenges.min(other.pending_challenges),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn frame(self, lane: Lane) -> usize {
        match lane {
            Lane::Control => self.control_frame as usize,
            Lane::Bulk => self.bulk_frame as usize,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerContext {
    pub session: [u8; 16],
    pub scope: Scope,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PeerBody {
    BulkReady {
        transfer: [u8; 16],
        proof: nf_identity::model::DeviceProof,
    },
    BeginBulk {
        descriptor: super::BulkDescriptor,
        nonce: [u8; 32],
        minimum_membership: u64,
    },
    BulkChallenge {
        transfer: [u8; 16],
        client_nonce: [u8; 32],
        server_nonce: [u8; 32],
        frontier: u64,
        challenge: [u8; 32],
    },
    ProveBulk {
        transfer: [u8; 16],
        nonce: [u8; 32],
        proof: nf_identity::model::DeviceProof,
    },
    BulkChunk {
        transfer: [u8; 16],
        index: u16,
        bytes: Vec<u8>,
    },
    BulkVerified {
        transfer: [u8; 16],
        total: u64,
        digest: [u8; 32],
        proof: nf_identity::model::DeviceProof,
    },
    BulkProgress {
        transfer: [u8; 16],
        next_index: u16,
        accepted_total: u64,
        proof: nf_identity::model::DeviceProof,
    },
    Unsupported {
        request: RequestId,
        reason: super::UnsupportedReason,
        proof: nf_identity::model::DeviceProof,
    },
    QueryChallenge {
        request: RequestId,
        client_nonce: [u8; 32],
        server_nonce: [u8; 32],
        frontier: u64,
        challenge: [u8; 32],
    },
    ProveQuery {
        request: RequestId,
        nonce: [u8; 32],
        proof: nf_identity::model::DeviceProof,
    },
    RetainedStatus {
        request: RequestId,
        phase: super::RetainedPhase,
        proof: nf_identity::model::DeviceProof,
    },
    ServerHello {
        nonce: [u8; 32],
        available: u32,
        selected_caps: u32,
        server_limits: PeerLimits,
        selected: PeerLimits,
        proof: nf_identity::model::DeviceProof,
    },
    ClientProof(nf_identity::model::DeviceProof),
    Finished(nf_identity::model::DeviceProof),
    Hello {
        account: AccountId,
        device: DeviceId,
        nonce: [u8; 32],
        required: u32,
        optional: u32,
        offered: PeerLimits,
    },
    BeginQuery {
        request: RequestId,
        nonce: [u8; 32],
        minimum_membership: u64,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerRecord {
    pub context: PeerContext,
    pub body: PeerBody,
}
