use super::{fields::*, reader::Writer, *};
use crate::PeerError;
pub(super) fn put(w: &mut Writer, body: &SyncBody) -> Result<(), PeerError> {
    match body {
        SyncBody::BeginDocument(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.export.0);
            w.raw(&v.document_digest);
            w.u32(v.total);
            w.u16(v.count);
            w.raw(&v.client_nonce);
        }
        SyncBody::DocumentChallenge(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.export.0);
            w.raw(&v.document_digest);
            w.raw(&v.client_nonce);
            w.raw(&v.server_nonce);
            put_stamp(w, v.membership);
            w.raw(&v.challenge);
        }
        SyncBody::ProveDocument(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.client_nonce);
            put_proof(w, &v.proof);
        }
        SyncBody::DocumentReady(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.export.0);
            w.raw(&v.document_digest);
            w.u32(v.total);
            w.u16(v.count);
            put_point(w, v.current);
            put_stamp(w, v.membership);
            put_proof(w, &v.proof);
        }
        SyncBody::SyncChunk(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.export.0);
            w.raw(&v.document_digest);
            w.u16(v.index);
            w.u16(v.count);
            w.u32(v.total);
            w.u16(v.data.len() as u16);
            w.raw(&v.data);
        }
        SyncBody::DocumentEnd(v) => {
            w.raw(&v.transfer.0);
            w.raw(&v.export.0);
            w.raw(&v.document_digest);
            w.u32(v.received);
            w.raw(&v.assembled_digest);
            put_point(w, v.current);
            put_stamp(w, v.membership);
            w.raw(&v.completion_nonce);
            put_proof(w, &v.proof);
        }
        _ => return Err(PeerError::Unsupported),
    }
    Ok(())
}
