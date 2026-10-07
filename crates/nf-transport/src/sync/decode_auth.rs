use super::{fields::*, reader::Reader, *};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::model::Scope;
pub(super) fn body(r: &mut Reader<'_>, kind: u8, scope: Scope) -> Result<SyncBody, PeerError> {
    Ok(match kind {
        1 => {
            if r.u8()? != 1 {
                return Err(PeerError::Unsupported);
            }
            SyncBody::Hello(SyncHello {
                account: AccountId::from_bytes(nonzero(r.array()?)?),
                device: DeviceId::from_bytes(nonzero(r.array()?)?),
                nonce: r.array()?,
                required: r.u32()?,
                optional: r.u32()?,
                offered: limits(r)?,
                pins: pins(r)?,
            })
        }
        2 => SyncBody::ServerHello(ServerHello {
            nonce: r.array()?,
            available: r.u32()?,
            selected_caps: r.u32()?,
            offered: limits(r)?,
            selected: limits(r)?,
            membership_digest: r.array()?,
            proof: proof(r, scope)?,
        }),
        3 => SyncBody::ClientProof(proof(r, scope)?),
        4 => SyncBody::Finished(proof(r, scope)?),
        _ => return Err(PeerError::Unsupported),
    })
}
