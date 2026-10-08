use super::{AcceptOffer, OfferId, TradeRejection};
use alloc::vec::Vec;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, OperationId, RequestId, UniverseId};
use sha2::{Digest, Sha256};

const ACCEPT: &[u8] = b"NF-TRADE-ACCEPT-OFFER-1\0";

/// Fixed 204-byte request; no client-supplied quantities or replacement terms.
pub fn accept_offer_bytes(value: &AcceptOffer) -> Vec<u8> {
    let mut bytes = Vec::from(ACCEPT);
    for id in [
        value.request.as_bytes(),
        value.operation.as_bytes(),
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&value.policy);
    bytes.extend_from_slice(value.offer.as_bytes());
    bytes.extend_from_slice(&value.version.to_le_bytes());
    bytes.extend_from_slice(&value.digest);
    bytes
}
pub fn accept_offer_digest(value: &AcceptOffer) -> [u8; 32] {
    Sha256::digest(accept_offer_bytes(value)).into()
}
/// An alias changes only RequestId; actor, device and exact original target remain bound.
pub fn economic_accept_offer_digest(value: &AcceptOffer) -> [u8; 32] {
    let bytes = accept_offer_bytes(value);
    let mut hash = Sha256::new();
    hash.update(b"NF-TRADE-ECONOMIC-ACCEPT-1\0");
    hash.update(&bytes[ACCEPT.len() + 16..]);
    hash.finalize().into()
}
pub fn decode_accept_offer(bytes: &[u8]) -> Result<AcceptOffer, TradeRejection> {
    if bytes.len() != 204 || !bytes.starts_with(ACCEPT) {
        return Err(TradeRejection::Malformed);
    }
    let mut reader = crate::supplies::record::Reader {
        bytes,
        offset: ACCEPT.len(),
    };
    let mut read = || -> Result<AcceptOffer, crate::supplies::SuppliesRejection> {
        Ok(AcceptOffer {
            request: RequestId::from_bytes(reader.take()?),
            operation: OperationId::from_bytes(reader.take()?),
            actor: AccountId::from_bytes(reader.take()?),
            device: DeviceId::from_bytes(reader.take()?),
            universe: UniverseId::from_bytes(reader.take()?),
            history: HistoryId::from_bytes(reader.take()?),
            policy: reader.take()?,
            offer: OfferId::from_bytes(reader.take()?),
            version: u32::from_le_bytes(reader.take()?),
            digest: reader.take()?,
        })
    };
    let value = read().map_err(|_| TradeRejection::Malformed)?;
    if reader.offset != bytes.len() || accept_offer_bytes(&value) != bytes {
        return Err(TradeRejection::Malformed);
    }
    Ok(value)
}
