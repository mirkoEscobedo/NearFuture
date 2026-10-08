use super::*;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, OperationId, RequestId, UniverseId};
use sha2::{Digest, Sha256};

const PREFIX: &[u8] = b"NF-SUPPLIES-ISSUANCE-1\0";

/// Economic identity excludes only RequestId, under a distinct closed domain.
pub fn economic_issuance_digest(value: &Issuance) -> [u8; 32] {
    let bytes = issuance_bytes(value);
    let mut hash = Sha256::new();
    hash.update(b"NF-SUPPLIES-ECONOMIC-ISSUE-1\0");
    hash.update(&bytes[PREFIX.len() + 16..]);
    hash.finalize().into()
}
pub fn origin_bytes(origin: Origin) -> [u8; 36] {
    let mut bytes = [0; 36];
    let class = match origin.trust {
        TrustClass::Sandbox => 1_u32,
        TrustClass::CooperativeAudited => 2,
        TrustClass::Canonical => 3,
    };
    bytes[..4].copy_from_slice(&class.to_le_bytes());
    bytes[4..].copy_from_slice(&origin.lineage);
    bytes
}
pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) offset: usize,
}
impl Reader<'_> {
    pub(crate) fn take<const N: usize>(&mut self) -> Result<[u8; N], SuppliesRejection> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or(SuppliesRejection::Malformed)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(SuppliesRejection::Malformed)?;
        self.offset = end;
        bytes.try_into().map_err(|_| SuppliesRejection::Malformed)
    }
}
/// Closed, fixed-size canonical persisted issuance; rejects trailing/unknown bytes.
pub fn decode_issuance(bytes: &[u8]) -> Result<Issuance, SuppliesRejection> {
    if !bytes.starts_with(PREFIX) {
        return Err(SuppliesRejection::Malformed);
    }
    let mut r = Reader {
        bytes,
        offset: PREFIX.len(),
    };
    let request = RequestId::from_bytes(r.take()?);
    let issuance = OperationId::from_bytes(r.take()?);
    let actor = AccountId::from_bytes(r.take()?);
    let device = DeviceId::from_bytes(r.take()?);
    let beneficiary = AccountId::from_bytes(r.take()?);
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
        1 => IssuanceReason::AuthorityGrant,
        _ => return Err(SuppliesRejection::Malformed),
    };
    let amount = u64::from_le_bytes(r.take()?);
    if r.offset != bytes.len() {
        return Err(SuppliesRejection::Malformed);
    }
    let value = Issuance {
        request,
        issuance,
        actor,
        device,
        beneficiary,
        universe,
        history,
        policy,
        content,
        origin,
        reason,
        amount,
    };
    if issuance_bytes(&value) != bytes {
        return Err(SuppliesRejection::Malformed);
    }
    Ok(value)
}
