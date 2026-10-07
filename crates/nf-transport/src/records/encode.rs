use super::{
    Lane, PeerBody, PeerLimits, PeerRecord, decode_body,
    fields::{write_limits, write_proof},
};
use crate::PeerError;
pub fn encode_body(
    record: &PeerRecord,
    lane: Lane,
    limits: PeerLimits,
) -> Result<Vec<u8>, PeerError> {
    limits.validate()?;
    if let PeerBody::BulkChunk { bytes, index, .. } = &record.body
        && (bytes.is_empty()
            || bytes.len() > limits.chunk_bytes as usize
            || *index >= 128
            || bytes.len() + 146 > limits.frame(lane))
    {
        return Err(PeerError::Limit);
    }
    let kind = match &record.body {
        PeerBody::BulkReady { .. } => 16,
        PeerBody::BeginBulk { .. } => 10,
        PeerBody::BulkChallenge { .. } => 11,
        PeerBody::ProveBulk { .. } => 12,
        PeerBody::BulkChunk { .. } => 13,
        PeerBody::BulkVerified { .. } => 14,
        PeerBody::BulkProgress { .. } => 15,
        PeerBody::Hello { .. } => 1,
        PeerBody::ServerHello { .. } => 2,
        PeerBody::ClientProof(_) => 3,
        PeerBody::Finished(_) => 4,
        PeerBody::BeginQuery { .. } => 5,
        PeerBody::RetainedStatus { .. } => 8,
        PeerBody::Unsupported { .. } => 9,
        PeerBody::QueryChallenge { .. } => 6,
        PeerBody::ProveQuery { .. } => 7,
    };
    let mut b = Vec::with_capacity(515);
    b.extend(b"NF-PEER-1\0");
    b.extend(1u16.to_le_bytes());
    b.extend([kind, lane as u8]);
    b.extend(record.context.session);
    b.extend(record.context.scope.universe.as_bytes());
    b.extend(record.context.scope.history.as_bytes());
    b.extend(record.context.ruleset);
    b.extend(record.context.content);
    match &record.body {
        PeerBody::BulkReady { .. }
        | PeerBody::BeginBulk { .. }
        | PeerBody::BulkChallenge { .. }
        | PeerBody::ProveBulk { .. }
        | PeerBody::BulkChunk { .. }
        | PeerBody::BulkVerified { .. }
        | PeerBody::BulkProgress { .. } => super::bulk::write(&mut b, &record.body)?,
        PeerBody::Hello {
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
        PeerBody::ServerHello {
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
        PeerBody::QueryChallenge {
            request,
            client_nonce,
            server_nonce,
            frontier,
            challenge,
        } => {
            b.extend(request.as_bytes());
            b.extend(client_nonce);
            b.extend(server_nonce);
            b.extend(frontier.to_le_bytes());
            b.extend(challenge);
        }
        PeerBody::ProveQuery {
            request,
            nonce,
            proof,
        } => {
            b.extend(request.as_bytes());
            b.extend(nonce);
            write_proof(&mut b, proof)?;
        }
        PeerBody::Unsupported {
            request,
            reason,
            proof,
        } => {
            b.extend(request.as_bytes());
            b.push(*reason as u8);
            write_proof(&mut b, proof)?;
        }
        PeerBody::RetainedStatus {
            request,
            phase,
            proof,
        } => {
            b.extend(request.as_bytes());
            super::status::write(&mut b, phase);
            write_proof(&mut b, proof)?;
        }
        PeerBody::ClientProof(p) | PeerBody::Finished(p) => write_proof(&mut b, p)?,
        PeerBody::BeginQuery {
            request,
            nonce,
            minimum_membership,
        } => {
            b.extend(request.as_bytes());
            b.extend(nonce);
            b.extend(minimum_membership.to_le_bytes());
        }
    }
    decode_body(&b, lane, limits)?;
    Ok(b)
}
