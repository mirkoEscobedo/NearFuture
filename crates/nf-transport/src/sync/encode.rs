use super::{reader::Writer, *};
use crate::PeerError;
pub fn encode_body(record: &SyncRecord, policy: SyncWirePolicy) -> Result<Vec<u8>, PeerError> {
    let n = super::validation::validate(record, policy)?;
    let mut w = Writer {
        bytes: Vec::with_capacity(n),
    };
    w.raw(b"NF-SYNC-1\0");
    w.u16(1);
    w.u8(record.body.kind());
    w.u8(match record.lane {
        SyncLane::Control => 1,
        SyncLane::Transfer => 2,
    });
    w.raw(&record.context.session);
    w.raw(record.context.scope.universe.as_bytes());
    w.raw(record.context.scope.history.as_bytes());
    w.raw(&record.context.ruleset);
    w.raw(&record.context.content);
    put_body(&mut w, &record.body)?;
    if w.bytes.len() != n {
        return Err(PeerError::Malformed);
    }
    Ok(w.bytes)
}
fn put_body(w: &mut Writer, body: &SyncBody) -> Result<(), PeerError> {
    match body {
        SyncBody::Hello(_)
        | SyncBody::ServerHello(_)
        | SyncBody::ClientProof(_)
        | SyncBody::Finished(_) => super::encode_auth::put(w, body),
        SyncBody::BeginSync(_)
        | SyncBody::SyncChallenge(_)
        | SyncBody::ProveSync(_)
        | SyncBody::ManifestOffer(_)
        | SyncBody::GapRequiresSnapshot(_)
        | SyncBody::ReplicaInstallReceipt(_)
        | SyncBody::SyncRefused(_) => super::encode_control::put(w, body),
        SyncBody::BeginDocument(_)
        | SyncBody::DocumentChallenge(_)
        | SyncBody::ProveDocument(_)
        | SyncBody::DocumentReady(_)
        | SyncBody::SyncChunk(_)
        | SyncBody::DocumentEnd(_) => super::encode_transfer::put(w, body),
    }
}
