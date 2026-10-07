use super::{fields::*, reader::Reader, *};
use crate::PeerError;
use nf_contract::identity::EventSeq;
use nf_identity::model::Scope;
pub(super) fn body(r: &mut Reader<'_>, kind: u8, scope: Scope) -> Result<SyncBody, PeerError> {
    Ok(match kind {
        5 => {
            let mode = match r.u8()? {
                1 => SyncMode::Delta,
                2 => SyncMode::Snapshot,
                _ => return Err(PeerError::Unsupported),
            };
            SyncBody::BeginSync(BeginSync {
                mode,
                pins: pins(r)?,
                client_nonce: r.array()?,
                minimum_event: EventSeq(r.u64()?),
                minimum_store: r.u64()?,
                minimum_membership: r.u64()?,
                base: optional_point(r)?,
                membership_digest: r.array()?,
                request: SyncRequestId(nonzero(r.array()?)?),
                maximum_documents: r.u16()?,
                maximum_data_bytes: r.u64()?,
            })
        }
        6 => SyncBody::SyncChallenge(SyncChallenge {
            request: SyncRequestId(nonzero(r.array()?)?),
            client_nonce: r.array()?,
            server_nonce: r.array()?,
            captured: point(r)?,
            membership: stamp(r)?,
            challenge: r.array()?,
        }),
        7 => SyncBody::ProveSync(ProveSync {
            request: SyncRequestId(nonzero(r.array()?)?),
            client_nonce: r.array()?,
            proof: proof(r, scope)?,
        }),
        8 => SyncBody::ManifestOffer(ManifestOffer {
            request: SyncRequestId(nonzero(r.array()?)?),
            export: ExportId(nonzero(r.array()?)?),
            target: point(r)?,
            manifest_digest: r.array()?,
            manifest_length: r.u32()?,
            document_count: r.u16()?,
            data_bytes: r.u64()?,
            current: point(r)?,
            membership: stamp(r)?,
            proof: proof(r, scope)?,
        }),
        9 => {
            let request = SyncRequestId(nonzero(r.array()?)?);
            let reason = match r.u8()? {
                1 => GapReason::PrefixPruned,
                2 => GapReason::BaseMismatch,
                _ => return Err(PeerError::Unsupported),
            };
            SyncBody::GapRequiresSnapshot(GapRequiresSnapshot {
                request,
                reason,
                oldest: optional_point(r)?,
                current: point(r)?,
                membership: stamp(r)?,
                proof: proof(r, scope)?,
            })
        }
        16 => SyncBody::ReplicaInstallReceipt(ReplicaInstallReceipt {
            export: ExportId(nonzero(r.array()?)?),
            manifest_digest: r.array()?,
            target: point(r)?,
            generation: r.u64()?,
            proof: proof(r, scope)?,
        }),
        17 => {
            let request = SyncRequestId(nonzero(r.array()?)?);
            let e = r.array()?;
            let export = if e == [0; 16] {
                None
            } else {
                Some(ExportId(e))
            };
            let reason = match r.u8()? {
                1 => RefusalReason::ExportPreparing,
                2 => RefusalReason::UnsupportedBase,
                3 => RefusalReason::SourceBelowMinima,
                4 => RefusalReason::Capacity,
                5 => RefusalReason::ExportExpired,
                _ => return Err(PeerError::Unsupported),
            };
            SyncBody::SyncRefused(SyncRefused {
                request,
                export,
                reason,
                current: point(r)?,
                membership: stamp(r)?,
                proof: proof(r, scope)?,
            })
        }
        _ => return Err(PeerError::Unsupported),
    })
}
