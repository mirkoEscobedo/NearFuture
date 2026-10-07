use super::{fields::*, reader::Reader, *};
use crate::PeerError;
use nf_identity::model::Scope;

pub(super) fn body(
    r: &mut Reader<'_>,
    kind: u8,
    scope: Scope,
    p: SyncWirePolicy,
) -> Result<SyncBody, PeerError> {
    Ok(match kind {
        10 => SyncBody::BeginDocument(BeginDocument {
            transfer: TransferId(nonzero(r.array()?)?),
            export: ExportId(nonzero(r.array()?)?),
            document_digest: r.array()?,
            total: r.u32()?,
            count: r.u16()?,
            client_nonce: r.array()?,
        }),
        11 => SyncBody::DocumentChallenge(DocumentChallenge {
            transfer: TransferId(nonzero(r.array()?)?),
            export: ExportId(nonzero(r.array()?)?),
            document_digest: r.array()?,
            client_nonce: r.array()?,
            server_nonce: r.array()?,
            membership: stamp(r)?,
            challenge: r.array()?,
        }),
        12 => SyncBody::ProveDocument(ProveDocument {
            transfer: TransferId(nonzero(r.array()?)?),
            client_nonce: r.array()?,
            proof: proof(r, scope)?,
        }),
        13 => SyncBody::DocumentReady(DocumentReady {
            transfer: TransferId(nonzero(r.array()?)?),
            export: ExportId(nonzero(r.array()?)?),
            document_digest: r.array()?,
            total: r.u32()?,
            count: r.u16()?,
            current: point(r)?,
            membership: stamp(r)?,
            proof: proof(r, scope)?,
        }),
        14 => {
            let transfer = TransferId(nonzero(r.array()?)?);
            let export = ExportId(nonzero(r.array()?)?);
            let document_digest = r.array()?;
            let index = r.u16()?;
            let count = r.u16()?;
            let total = r.u32()?;
            let n = usize::from(r.u16()?);
            // Validate descriptor, selected count/remainder and length before any data allocation.
            p.limits.chunk(total, count, index, n)?;
            let data = r.take(n)?.to_vec();
            SyncBody::SyncChunk(SyncChunk {
                transfer,
                export,
                document_digest,
                index,
                count,
                total,
                data,
            })
        }
        15 => SyncBody::DocumentEnd(DocumentEnd {
            transfer: TransferId(nonzero(r.array()?)?),
            export: ExportId(nonzero(r.array()?)?),
            document_digest: r.array()?,
            received: r.u32()?,
            assembled_digest: r.array()?,
            current: point(r)?,
            membership: stamp(r)?,
            completion_nonce: r.array()?,
            proof: proof(r, scope)?,
        }),
        _ => return Err(PeerError::Unsupported),
    })
}
