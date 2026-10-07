use super::{fields::*, reader::Writer, *};
use crate::PeerError;
pub(super) fn put(w: &mut Writer, body: &SyncBody) -> Result<(), PeerError> {
    match body {
        SyncBody::Hello(v) => {
            w.u8(1);
            w.raw(v.account.as_bytes());
            w.raw(v.device.as_bytes());
            w.raw(&v.nonce);
            w.u32(v.required);
            w.u32(v.optional);
            put_limits(w, v.offered);
            put_pins(w, v.pins);
        }
        SyncBody::ServerHello(v) => {
            w.raw(&v.nonce);
            w.u32(v.available);
            w.u32(v.selected_caps);
            put_limits(w, v.offered);
            put_limits(w, v.selected);
            w.raw(&v.membership_digest);
            put_proof(w, &v.proof);
        }
        SyncBody::ClientProof(v) | SyncBody::Finished(v) => put_proof(w, v),
        _ => return Err(PeerError::Unsupported),
    }
    Ok(())
}
