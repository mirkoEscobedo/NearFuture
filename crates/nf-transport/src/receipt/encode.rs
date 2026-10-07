use super::*;
use crate::{
    PeerError,
    records::{
        PeerLimits,
        fields::{write_limits, write_proof},
    },
};
pub fn encode_body(r: &ReceiptRecord, limits: PeerLimits) -> Result<Vec<u8>, PeerError> {
    limits.validate()?;
    let mut b = Vec::with_capacity(MAX_BODY_BYTES);
    b.extend(b"NF-PEER-2\0");
    b.extend(2u16.to_le_bytes());
    b.extend([r.body.kind(), 1]);
    b.extend(r.context.session);
    b.extend(r.context.scope.universe.as_bytes());
    b.extend(r.context.scope.history.as_bytes());
    b.extend(r.context.ruleset);
    b.extend(r.context.content);
    match &r.body {
        ReceiptBody::Hello {
            account,
            device,
            nonce,
            required,
            optional,
            offered,
        } => {
            b.extend(account.as_bytes());
            b.extend(device.as_bytes());
            b.extend(nonce);
            b.extend(required.to_le_bytes());
            b.extend(optional.to_le_bytes());
            write_limits(&mut b, *offered);
        }
        ReceiptBody::ServerHello {
            nonce,
            available,
            selected_caps,
            server_limits,
            selected,
            proof,
        } => {
            b.extend(nonce);
            b.extend(available.to_le_bytes());
            b.extend(selected_caps.to_le_bytes());
            write_limits(&mut b, *server_limits);
            write_limits(&mut b, *selected);
            write_proof(&mut b, proof)?;
        }
        ReceiptBody::ClientProof(p) | ReceiptBody::Finished(p) => write_proof(&mut b, p)?,
        ReceiptBody::Begin {
            target,
            nonce,
            minimum,
        } => {
            write_target(&mut b, *target);
            b.extend(nonce);
            b.extend(minimum.membership_revision.to_le_bytes());
            b.extend(minimum.event.0.to_le_bytes());
            b.extend(minimum.store_revision.to_le_bytes());
        }
        ReceiptBody::Challenge {
            target,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } => {
            write_target(&mut b, *target);
            b.extend(client_nonce);
            b.extend(server_nonce);
            b.extend(frontier.to_le_bytes());
            b.extend(challenge);
        }
        ReceiptBody::Prove {
            request,
            nonce,
            proof,
        } => {
            b.extend(request.as_bytes());
            b.extend(nonce);
            write_proof(&mut b, proof)?;
        }
        ReceiptBody::Status { status, proof } => {
            write_status(&mut b, status);
            write_proof(&mut b, proof)?;
        }
        ReceiptBody::Unsupported {
            request,
            reason,
            current,
            proof,
        } => {
            b.extend(request.as_bytes());
            b.push(*reason as u8);
            b.extend(RECEIPT_PROFILE.to_le_bytes());
            write_current(&mut b, *current);
            write_proof(&mut b, proof)?;
        }
    }
    decode_body(&b, limits)?;
    Ok(b)
}
fn write_target(b: &mut Vec<u8>, t: ReceiptTarget) {
    b.extend(t.request.as_bytes());
    b.extend(t.operation.as_bytes());
    b.extend(t.binding);
    b.extend(RECEIPT_PROFILE.to_le_bytes());
}
fn write_current(b: &mut Vec<u8>, c: SourceMinima) {
    b.extend(c.store_revision.to_le_bytes());
    b.extend(c.event.0.to_le_bytes());
    b.extend(c.membership_revision.to_le_bytes());
}
fn write_status(b: &mut Vec<u8>, s: &ReceiptStatus) {
    b.extend(s.request.as_bytes());
    b.extend(s.account.as_bytes());
    b.extend(s.device.as_bytes());
    b.extend(s.operation.as_bytes());
    b.extend(s.binding);
    let (phase, present, sequence, rejection) = match s.phase {
        ReceiptPhase::Unknown => (1, 0, 0, 0),
        ReceiptPhase::Pending => (2, 0, 0, 0),
        ReceiptPhase::Rejected { sequence } => (3, 1, sequence.0, 1),
        ReceiptPhase::Committed { sequence } => (4, 1, sequence.0, 0),
    };
    b.extend([phase, present]);
    b.extend(sequence.to_le_bytes());
    b.push(rejection);
    b.extend(RECEIPT_PROFILE.to_le_bytes());
    write_current(b, s.current);
}
/// Hash of the exact admitted header and result prefix. This grants no authorization.
pub fn reply_prefix_digest(r: &ReceiptRecord, l: PeerLimits) -> Result<[u8; 32], PeerError> {
    use sha2::{Digest, Sha256};
    if !matches!(
        r.body,
        ReceiptBody::Status { .. } | ReceiptBody::Unsupported { .. }
    ) {
        return Err(PeerError::Unsupported);
    }
    let b = encode_body(r, l)?;
    Ok(Sha256::digest(&b[..b.len() - 297]).into())
}
