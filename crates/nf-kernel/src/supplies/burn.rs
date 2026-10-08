use super::*;
use alloc::vec::Vec;
use nf_contract::canonical::binding::RequestBinding;
use sha2::{Digest, Sha256};

pub fn burn_bytes(value: &Burn) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"NF-SUPPLIES-BURN-1\0");
    for id in [
        value.request.as_bytes(),
        value.burn.as_bytes(),
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.owner.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&value.policy);
    bytes.extend_from_slice(&value.content);
    bytes.extend_from_slice(&origin_bytes(value.origin));
    bytes.extend_from_slice(
        &(match value.reason {
            BurnReason::AuthorityDestruction => 1_u32,
        })
        .to_le_bytes(),
    );
    bytes.extend_from_slice(&value.amount.to_le_bytes());
    bytes
}
pub fn burn_digest(value: &Burn) -> [u8; 32] {
    Sha256::digest(burn_bytes(value)).into()
}
pub fn burn_binding(value: &Burn) -> RequestBinding {
    RequestBinding {
        request_id: value.request,
        account_id: value.actor,
        device_id: value.device,
        universe_id: value.universe,
        history_id: value.history,
        operation_kind: BURN_OPERATION,
        payload_digest: burn_digest(value),
    }
}

const PREFIX: &[u8] = b"NF-SUPPLIES-BURN-1\0";
/// Economic identity excludes only RequestId and is distinct from issuance.
pub fn economic_burn_digest(value: &Burn) -> [u8; 32] {
    let bytes = burn_bytes(value);
    let mut hash = Sha256::new();
    hash.update(b"NF-SUPPLIES-ECONOMIC-BURN-1\0");
    hash.update(&bytes[PREFIX.len() + 16..]);
    hash.finalize().into()
}
/// Closed canonical burn record; unknown reason/trust or trailing bytes are rejected.
pub fn decode_burn(bytes: &[u8]) -> Result<Burn, SuppliesRejection> {
    use nf_contract::identity::*;
    if !bytes.starts_with(PREFIX) {
        return Err(SuppliesRejection::Malformed);
    }
    let mut r = super::record::Reader {
        bytes,
        offset: PREFIX.len(),
    };
    let request = RequestId::from_bytes(r.take()?);
    let burn = OperationId::from_bytes(r.take()?);
    let actor = AccountId::from_bytes(r.take()?);
    let device = DeviceId::from_bytes(r.take()?);
    let owner = AccountId::from_bytes(r.take()?);
    let universe = UniverseId::from_bytes(r.take()?);
    let history = HistoryId::from_bytes(r.take()?);
    let policy = r.take()?;
    let content = r.take()?;
    let trust = match u32::from_le_bytes(r.take()?) {
        1 => TrustClass::Sandbox,
        2 => TrustClass::CooperativeAudited,
        3 => TrustClass::Canonical,
        _ => return Err(SuppliesRejection::Malformed),
    };
    let origin = Origin {
        trust,
        lineage: r.take()?,
    };
    let reason = match u32::from_le_bytes(r.take()?) {
        1 => BurnReason::AuthorityDestruction,
        _ => return Err(SuppliesRejection::Malformed),
    };
    let amount = u64::from_le_bytes(r.take()?);
    if r.offset != bytes.len() {
        return Err(SuppliesRejection::Malformed);
    }
    let value = Burn {
        request,
        burn,
        actor,
        device,
        owner,
        universe,
        history,
        policy,
        content,
        origin,
        reason,
        amount,
    };
    if burn_bytes(&value) != bytes {
        return Err(SuppliesRejection::Malformed);
    }
    Ok(value)
}
