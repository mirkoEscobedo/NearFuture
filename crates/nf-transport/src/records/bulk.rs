use crate::PeerError;
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BulkDescriptor {
    pub transfer: [u8; 16],
    pub total: u64,
    pub chunks: u16,
    pub digest: [u8; 32],
}
impl BulkDescriptor {
    pub fn validate(self, limits: super::PeerLimits) -> Result<(), PeerError> {
        limits.validate()?;
        if self.total == 0
            || self.total > 1_048_576
            || self.chunks == 0
            || self.chunks > 128
            || self.total > self.chunks as u64 * limits.chunk_bytes as u64
            || self.total < self.chunks as u64
        {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn digest(self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.transfer);
        h.update(self.total.to_le_bytes());
        h.update(self.chunks.to_le_bytes());
        h.update(self.digest);
        h.finalize().into()
    }
}
use super::{PeerBody, PeerLimits, Reader, fields};
use nf_identity::model::Scope;
pub(super) fn read(
    kind: u8,
    r: &mut Reader<'_>,
    scope: Scope,
    limits: PeerLimits,
) -> Result<PeerBody, PeerError> {
    Ok(match kind {
        16 => PeerBody::BulkReady {
            transfer: r.array()?,
            proof: fields::read_proof(r, scope)?,
        },
        10 => {
            let d = BulkDescriptor {
                transfer: r.array()?,
                total: r.u64()?,
                chunks: r.u16()?,
                digest: r.array()?,
            };
            d.validate(limits)?;
            PeerBody::BeginBulk {
                descriptor: d,
                nonce: r.array()?,
                minimum_membership: r.u64()?,
            }
        }
        11 => PeerBody::BulkChallenge {
            transfer: r.array()?,
            client_nonce: r.array()?,
            server_nonce: r.array()?,
            frontier: r.u64()?,
            challenge: r.array()?,
        },
        12 => PeerBody::ProveBulk {
            transfer: r.array()?,
            nonce: r.array()?,
            proof: fields::read_proof(r, scope)?,
        },
        13 => {
            let transfer = r.array()?;
            let index = r.u16()?;
            let n = r.u16()? as usize;
            if index >= 128 || n == 0 || n > limits.chunk_bytes as usize {
                return Err(PeerError::Limit);
            }
            let end = r.position.checked_add(n).ok_or(PeerError::Limit)?;
            let bytes = r
                .input
                .get(r.position..end)
                .ok_or(PeerError::Malformed)?
                .to_vec();
            r.position = end;
            PeerBody::BulkChunk {
                transfer,
                index,
                bytes,
            }
        }
        14 => {
            let transfer = r.array()?;
            let total = r.u64()?;
            if total == 0 || total > 1_048_576 {
                return Err(PeerError::Limit);
            }
            PeerBody::BulkVerified {
                transfer,
                total,
                digest: r.array()?,
                proof: fields::read_proof(r, scope)?,
            }
        }
        15 => {
            let transfer = r.array()?;
            let next_index = r.u16()?;
            let accepted_total = r.u64()?;
            if next_index == 0
                || next_index >= 128
                || accepted_total == 0
                || accepted_total > 1_048_576
            {
                return Err(PeerError::Limit);
            }
            PeerBody::BulkProgress {
                transfer,
                next_index,
                accepted_total,
                proof: fields::read_proof(r, scope)?,
            }
        }
        _ => return Err(PeerError::Unsupported),
    })
}
pub(super) fn write(b: &mut Vec<u8>, body: &PeerBody) -> Result<(), PeerError> {
    match body {
        PeerBody::BulkReady { transfer, proof } => {
            b.extend(transfer);
            fields::write_proof(b, proof)?;
        }
        PeerBody::BeginBulk {
            descriptor: d,
            nonce,
            minimum_membership,
        } => {
            b.extend(d.transfer);
            b.extend(d.total.to_le_bytes());
            b.extend(d.chunks.to_le_bytes());
            b.extend(d.digest);
            b.extend(nonce);
            b.extend(minimum_membership.to_le_bytes());
        }
        PeerBody::BulkChallenge {
            transfer,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } => {
            b.extend(transfer);
            b.extend(client_nonce);
            b.extend(server_nonce);
            b.extend(frontier.to_le_bytes());
            b.extend(challenge);
        }
        PeerBody::ProveBulk {
            transfer,
            nonce,
            proof,
        } => {
            b.extend(transfer);
            b.extend(nonce);
            fields::write_proof(b, proof)?;
        }
        PeerBody::BulkChunk {
            transfer,
            index,
            bytes,
        } => {
            b.extend(transfer);
            b.extend(index.to_le_bytes());
            b.extend((bytes.len() as u16).to_le_bytes());
            b.extend(bytes);
        }
        PeerBody::BulkVerified {
            transfer,
            total,
            digest,
            proof,
        } => {
            b.extend(transfer);
            b.extend(total.to_le_bytes());
            b.extend(digest);
            fields::write_proof(b, proof)?;
        }
        PeerBody::BulkProgress {
            transfer,
            next_index,
            accepted_total,
            proof,
        } => {
            b.extend(transfer);
            b.extend(next_index.to_le_bytes());
            b.extend(accepted_total.to_le_bytes());
            fields::write_proof(b, proof)?;
        }
        _ => return Err(PeerError::Unsupported),
    }
    Ok(())
}
