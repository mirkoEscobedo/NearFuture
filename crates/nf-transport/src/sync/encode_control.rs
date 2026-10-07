use super::{fields::*, reader::Writer, *};
use crate::PeerError;
pub(super) fn put(w: &mut Writer, body: &SyncBody) -> Result<(), PeerError> {
    match body {
        SyncBody::BeginSync(v) => {
            w.u8(match v.mode {
                SyncMode::Delta => 1,
                SyncMode::Snapshot => 2,
            });
            put_pins(w, v.pins);
            w.raw(&v.client_nonce);
            w.u64(v.minimum_event.0);
            w.u64(v.minimum_store);
            w.u64(v.minimum_membership);
            put_optional_point(w, v.base);
            w.raw(&v.membership_digest);
            w.raw(&v.request.0);
            w.u16(v.maximum_documents);
            w.u64(v.maximum_data_bytes);
        }
        SyncBody::SyncChallenge(v) => {
            w.raw(&v.request.0);
            w.raw(&v.client_nonce);
            w.raw(&v.server_nonce);
            put_point(w, v.captured);
            put_stamp(w, v.membership);
            w.raw(&v.challenge);
        }
        SyncBody::ProveSync(v) => {
            w.raw(&v.request.0);
            w.raw(&v.client_nonce);
            put_proof(w, &v.proof);
        }
        SyncBody::ManifestOffer(v) => {
            w.raw(&v.request.0);
            w.raw(&v.export.0);
            put_point(w, v.target);
            w.raw(&v.manifest_digest);
            w.u32(v.manifest_length);
            w.u16(v.document_count);
            w.u64(v.data_bytes);
            put_point(w, v.current);
            put_stamp(w, v.membership);
            put_proof(w, &v.proof);
        }
        SyncBody::GapRequiresSnapshot(v) => {
            w.raw(&v.request.0);
            w.u8(match v.reason {
                GapReason::PrefixPruned => 1,
                GapReason::BaseMismatch => 2,
            });
            put_optional_point(w, v.oldest);
            put_point(w, v.current);
            put_stamp(w, v.membership);
            put_proof(w, &v.proof);
        }
        SyncBody::ReplicaInstallReceipt(v) => {
            w.raw(&v.export.0);
            w.raw(&v.manifest_digest);
            put_point(w, v.target);
            w.u64(v.generation);
            put_proof(w, &v.proof);
        }
        SyncBody::SyncRefused(v) => {
            w.raw(&v.request.0);
            w.raw(&v.export.map_or([0; 16], |e| e.0));
            w.u8(match v.reason {
                RefusalReason::ExportPreparing => 1,
                RefusalReason::UnsupportedBase => 2,
                RefusalReason::SourceBelowMinima => 3,
                RefusalReason::Capacity => 4,
                RefusalReason::ExportExpired => 5,
            });
            put_point(w, v.current);
            put_stamp(w, v.membership);
            put_proof(w, &v.proof);
        }
        _ => return Err(PeerError::Unsupported),
    }
    Ok(())
}
