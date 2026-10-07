use super::{super::*, support::*};
pub fn body(r: &Row, kind: u8) -> SyncBody {
    let maximum = r.name().contains("maximum");
    let total = if maximum { 8192 } else { 391 };
    let document = vec![if maximum { 0x2a } else { 0x28 }; total];
    let digest = hash(&document);
    let count = (total as u32).div_ceil(r.policy().limits.chunk_bytes) as u16;
    match kind {
        10 => SyncBody::BeginDocument(BeginDocument {
            transfer: TransferId([16; 16]),
            export: ExportId([15; 16]),
            document_digest: digest,
            total: total as u32,
            count,
            client_nonce: [30; 32],
        }),
        11 => SyncBody::DocumentChallenge(DocumentChallenge {
            transfer: TransferId([16; 16]),
            export: ExportId([15; 16]),
            document_digest: digest,
            client_nonce: [30; 32],
            server_nonce: [31; 32],
            membership: stamp(),
            challenge: transcript("prove-document-lane2-transcript"),
        }),
        12 => SyncBody::ProveDocument(ProveDocument {
            transfer: TransferId([16; 16]),
            client_nonce: [30; 32],
            proof: proof(r),
        }),
        13 => SyncBody::DocumentReady(DocumentReady {
            transfer: TransferId([16; 16]),
            export: ExportId([15; 16]),
            document_digest: digest,
            total: total as u32,
            count,
            current: point(2),
            membership: stamp(),
            proof: proof(r),
        }),
        14 => {
            let index = if r.name().contains("final") { 1 } else { 0 };
            let start = index as usize * r.policy().limits.chunk_bytes as usize;
            let end = total.min(start + r.policy().limits.chunk_bytes as usize);
            SyncBody::SyncChunk(SyncChunk {
                transfer: TransferId([16; 16]),
                export: ExportId([15; 16]),
                document_digest: digest,
                index,
                count,
                total: total as u32,
                data: document[start..end].to_vec(),
            })
        }
        15 => SyncBody::DocumentEnd(DocumentEnd {
            transfer: TransferId([16; 16]),
            export: ExportId([15; 16]),
            document_digest: digest,
            received: total as u32,
            assembled_digest: digest,
            current: point(2),
            membership: stamp(),
            completion_nonce: [19; 32],
            proof: proof(r),
        }),
        16 => SyncBody::ReplicaInstallReceipt(ReplicaInstallReceipt {
            export: ExportId([15; 16]),
            manifest_digest: [28; 32],
            target: point(1),
            generation: 2,
            proof: proof(r),
        }),
        _ => panic!("closed transfer fixture"),
    }
}
