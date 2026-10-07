use super::{fields::*, reader::Reader, *};
use crate::{PeerError, records::PeerContext};
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::model::Scope;
pub fn decode_body(input: &[u8], policy: SyncWirePolicy) -> Result<SyncRecord, PeerError> {
    if !(HEADER_BYTES..=policy.frame_maximum()?).contains(&input.len()) {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { input, position: 0 };
    if r.array::<10>()? != *b"NF-SYNC-1\0" || r.u16()? != 1 {
        return Err(PeerError::Unsupported);
    }
    let kind = r.u8()?;
    let lane = match r.u8()? {
        1 => SyncLane::Control,
        2 => SyncLane::Transfer,
        _ => return Err(PeerError::Unsupported),
    };
    if lane != policy.lane {
        return Err(PeerError::Scope);
    }
    super::validation::lane(kind, lane)?;
    let context = PeerContext {
        session: r.array()?,
        scope: Scope {
            universe: UniverseId::from_bytes(nonzero(r.array()?)?),
            history: HistoryId::from_bytes(nonzero(r.array()?)?),
        },
        ruleset: r.array()?,
        content: r.array()?,
    };
    let body = body(&mut r, kind, context.scope, policy)?;
    r.finish()?;
    let record = SyncRecord {
        lane,
        context,
        body,
    };
    super::validation::validate(&record, policy)?;
    Ok(record)
}
fn body(
    r: &mut Reader<'_>,
    kind: u8,
    scope: Scope,
    p: SyncWirePolicy,
) -> Result<SyncBody, PeerError> {
    match kind {
        1..=4 => super::decode_auth::body(r, kind, scope),
        5..=9 | 16..=17 => super::decode_control::body(r, kind, scope),
        10..=15 => super::decode_transfer::body(r, kind, scope, p),
        _ => Err(PeerError::Unsupported),
    }
}
