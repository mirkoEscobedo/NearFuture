use super::*;
use crate::supplies::{TrustClass, origin_bytes, policy_bytes};
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
const OFFER: &[u8] = b"NF-TRADE-OFFER-TERMS-1\0";
const RESERVE: &[u8] = b"NF-TRADE-RESERVE-OFFER-1\0";
fn asset(bytes: &mut Vec<u8>, value: AssetTerms) {
    bytes.extend_from_slice(&value.content);
    bytes.extend_from_slice(&origin_bytes(value.origin));
    bytes.extend_from_slice(&value.amount.to_le_bytes());
}
pub fn trade_policy_bytes(policy: &TradePolicy) -> Result<Vec<u8>, TradeRejection> {
    policy.validate()?;
    let supplies = policy_bytes(&policy.supplies).map_err(|_| TradeRejection::Policy)?;
    let mut bytes = Vec::from(b"NF-TRADE-POLICY-2\0".as_slice());
    bytes.extend_from_slice(&(supplies.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&supplies);
    bytes.extend_from_slice(policy.clock_authority.as_bytes());
    let admission = match policy.admission {
        TradeAdmission::AuthorityVaultOnly => 1_u32,
    };
    bytes.extend_from_slice(&admission.to_le_bytes());
    let cancel_rule = match policy.cancel_rule {
        CancelRule::EitherNamedParty => 1_u32,
    };
    bytes.extend_from_slice(&cancel_rule.to_le_bytes());
    bytes.extend_from_slice(&(policy.allowed_origins.len() as u32).to_le_bytes());
    for origin in &policy.allowed_origins {
        bytes.extend_from_slice(&origin_bytes(*origin));
    }
    Ok(bytes)
}
pub fn trade_policy_digest(policy: &TradePolicy) -> Result<[u8; 32], TradeRejection> {
    Ok(Sha256::digest(trade_policy_bytes(policy)?).into())
}
pub fn offer_terms_bytes(value: &OfferTerms) -> Vec<u8> {
    let mut bytes = Vec::from(OFFER);
    bytes.extend_from_slice(value.offer.as_bytes());
    bytes.extend_from_slice(&value.version.to_le_bytes());
    for id in [
        value.maker.as_bytes(),
        value.taker.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&value.policy);
    asset(&mut bytes, value.give);
    asset(&mut bytes, value.want);
    bytes.extend_from_slice(&value.expires_at.to_le_bytes());
    bytes
}
pub fn offer_terms_digest(value: &OfferTerms) -> [u8; 32] {
    Sha256::digest(offer_terms_bytes(value)).into()
}
pub fn reserve_offer_bytes(value: &ReserveOffer) -> Vec<u8> {
    let mut bytes = Vec::from(RESERVE);
    for id in [
        value.request.as_bytes(),
        value.operation.as_bytes(),
        value.maker_device.as_bytes(),
        value.taker_device.as_bytes(),
    ] {
        bytes.extend_from_slice(id);
    }
    bytes.extend_from_slice(&offer_terms_bytes(&value.terms));
    bytes
}
pub fn reserve_offer_digest(value: &ReserveOffer) -> [u8; 32] {
    Sha256::digest(reserve_offer_bytes(value)).into()
}
pub fn economic_reserve_offer_digest(value: &ReserveOffer) -> [u8; 32] {
    let bytes = reserve_offer_bytes(value);
    let mut hash = Sha256::new();
    hash.update(b"NF-TRADE-ECONOMIC-RESERVE-1\0");
    hash.update(&bytes[RESERVE.len() + 16..]);
    hash.finalize().into()
}
pub fn offer_status_digest(value: &OfferStatusQuery) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NF-TRADE-OFFER-STATUS-1\0");
    for id in [
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
        value.offer.as_bytes(),
    ] {
        hash.update(id);
    }
    hash.update(value.version.to_le_bytes());
    hash.finalize().into()
}
pub fn outbox_digest(value: &TradeOutboxQuery) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NF-TRADE-OUTBOX-QUERY-1\0");
    for id in [
        value.actor.as_bytes(),
        value.device.as_bytes(),
        value.universe.as_bytes(),
        value.history.as_bytes(),
    ] {
        hash.update(id);
    }
    hash.update(value.after_revision.to_le_bytes());
    hash.update(value.limit.to_le_bytes());
    hash.finalize().into()
}
pub fn decode_reserve_offer(bytes: &[u8]) -> Result<ReserveOffer, TradeRejection> {
    use nf_contract::identity::*;
    if bytes.len() != RESERVE.len() + 64 + OFFER.len() + 276 || !bytes.starts_with(RESERVE) {
        return Err(TradeRejection::Malformed);
    }
    let mut r = crate::supplies::record::Reader {
        bytes,
        offset: RESERVE.len(),
    };
    let mut take = || -> Result<ReserveOffer, crate::supplies::SuppliesRejection> {
        let request = RequestId::from_bytes(r.take()?);
        let operation = OperationId::from_bytes(r.take()?);
        let maker_device = DeviceId::from_bytes(r.take()?);
        let taker_device = DeviceId::from_bytes(r.take()?);
        if &r.take::<23>()?[..] != OFFER {
            return Err(crate::supplies::SuppliesRejection::Malformed);
        }
        let offer = OfferId::from_bytes(r.take()?);
        let version = u32::from_le_bytes(r.take()?);
        let maker = AccountId::from_bytes(r.take()?);
        let taker = AccountId::from_bytes(r.take()?);
        let universe = UniverseId::from_bytes(r.take()?);
        let history = HistoryId::from_bytes(r.take()?);
        let policy = r.take()?;
        let mut read_asset = || -> Result<AssetTerms, crate::supplies::SuppliesRejection> {
            let content = r.take()?;
            let trust = match u32::from_le_bytes(r.take()?) {
                1 => TrustClass::Sandbox,
                2 => TrustClass::CooperativeAudited,
                3 => TrustClass::Canonical,
                _ => return Err(crate::supplies::SuppliesRejection::Malformed),
            };
            let origin = crate::supplies::Origin {
                trust,
                lineage: r.take()?,
            };
            Ok(AssetTerms {
                content,
                origin,
                amount: u64::from_le_bytes(r.take()?),
            })
        };
        let give = read_asset()?;
        let want = read_asset()?;
        let expires_at = u64::from_le_bytes(r.take()?);
        Ok(ReserveOffer {
            request,
            operation,
            maker_device,
            taker_device,
            terms: OfferTerms {
                offer,
                version,
                maker,
                taker,
                universe,
                history,
                policy,
                give,
                want,
                expires_at,
            },
        })
    };
    let value = take().map_err(|_| TradeRejection::Malformed)?;
    if r.offset != bytes.len() || reserve_offer_bytes(&value) != bytes {
        return Err(TradeRejection::Malformed);
    }
    Ok(value)
}
