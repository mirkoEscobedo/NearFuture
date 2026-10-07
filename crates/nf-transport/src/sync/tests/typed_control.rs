use super::{super::*, support::*};
use nf_contract::identity::{AccountId, DeviceId, EventSeq};
pub fn body(r: &Row, kind: u8) -> SyncBody {
    match kind {
        1 => SyncBody::Hello(SyncHello {
            account: AccountId::from_bytes([1; 16]),
            device: DeviceId::from_bytes([2; 16]),
            nonce: [10; 32],
            required: 1,
            optional: 0,
            offered: SyncLimits::default(),
            pins: pins(),
        }),
        2 => SyncBody::ServerHello(ServerHello {
            nonce: [11; 32],
            available: 1,
            selected_caps: 1,
            offered: SyncLimits::default(),
            selected: SyncLimits::default(),
            membership_digest: [13; 32],
            proof: proof(r),
        }),
        3 => SyncBody::ClientProof(proof(r)),
        4 => SyncBody::Finished(proof(r)),
        5 => {
            let snapshot = r.name().contains("snapshot");
            SyncBody::BeginSync(BeginSync {
                mode: if snapshot {
                    SyncMode::Snapshot
                } else {
                    SyncMode::Delta
                },
                pins: pins(),
                client_nonce: [17; 32],
                minimum_event: EventSeq(90),
                minimum_store: 190,
                minimum_membership: 11,
                base: if snapshot { None } else { Some(point(0)) },
                membership_digest: [13; 32],
                request: SyncRequestId([14; 16]),
                maximum_documents: 257,
                maximum_data_bytes: 8388608,
            })
        }
        6 => SyncBody::SyncChallenge(SyncChallenge {
            request: SyncRequestId([14; 16]),
            client_nonce: [17; 32],
            server_nonce: [18; 32],
            captured: point(1),
            membership: stamp(),
            challenge: transcript("prove-sync-lane1-transcript"),
        }),
        7 => SyncBody::ProveSync(ProveSync {
            request: SyncRequestId([14; 16]),
            client_nonce: [17; 32],
            proof: proof(r),
        }),
        8 => SyncBody::ManifestOffer(ManifestOffer {
            request: SyncRequestId([14; 16]),
            export: ExportId([15; 16]),
            target: point(1),
            manifest_digest: [28; 32],
            manifest_length: 620,
            document_count: 1,
            data_bytes: 391,
            current: point(2),
            membership: stamp(),
            proof: proof(r),
        }),
        9 => SyncBody::GapRequiresSnapshot(GapRequiresSnapshot {
            request: SyncRequestId([14; 16]),
            reason: GapReason::PrefixPruned,
            oldest: Some(point(0)),
            current: point(2),
            membership: stamp(),
            proof: proof(r),
        }),
        10..=16 => super::typed_transfer::body(r, kind),
        17 => SyncBody::SyncRefused(SyncRefused {
            request: SyncRequestId([14; 16]),
            export: None,
            reason: RefusalReason::Capacity,
            current: point(2),
            membership: stamp(),
            proof: proof(r),
        }),
        _ => panic!("closed fixture kind"),
    }
}
