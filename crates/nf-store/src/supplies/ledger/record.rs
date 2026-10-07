use super::*;
use nf_kernel::supplies::{
    Burn, Reserve, SUPPLIES_CONTENT, burn_binding, burn_bytes, decode_burn, decode_issuance,
    decode_reserve, economic_burn_digest, economic_issuance_digest, economic_reserve_digest,
    issuance_binding, issuance_bytes, policy_digest, reserve_binding, reserve_bytes,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::supplies) enum Operation {
    Issue(Issuance),
    Burn(Burn),
    Reserve(Reserve),
}
impl Operation {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.starts_with(b"NF-SUPPLIES-ISSUANCE-1\0") {
            decode_issuance(bytes)
                .map(Self::Issue)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else if bytes.starts_with(b"NF-SUPPLIES-BURN-1\0") {
            decode_burn(bytes)
                .map(Self::Burn)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else if bytes.starts_with(b"NF-SUPPLIES-RESERVE-1\0") {
            decode_reserve(bytes)
                .map(Self::Reserve)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else {
            Err(SuppliesStoreError::Corrupt)
        }
    }
    pub fn id(self) -> OperationId {
        match self {
            Self::Issue(v) => v.issuance,
            Self::Burn(v) => v.burn,
            Self::Reserve(v) => v.reservation,
        }
    }
    pub fn request(self) -> RequestId {
        match self {
            Self::Issue(v) => v.request,
            Self::Burn(v) => v.request,
            Self::Reserve(v) => v.request,
        }
    }
    pub fn key(self) -> Key {
        match self {
            Self::Issue(v) => (v.beneficiary, v.content, v.origin),
            Self::Burn(v) => (v.owner, v.content, v.origin),
            Self::Reserve(v) => (v.owner, v.content, v.origin),
        }
    }
    pub fn binding(self) -> [u8; 32] {
        match self {
            Self::Issue(v) => issuance_binding(&v).digest(),
            Self::Burn(v) => burn_binding(&v).digest(),
            Self::Reserve(v) => reserve_binding(&v).digest(),
        }
    }
    pub fn economic(self) -> [u8; 32] {
        match self {
            Self::Issue(v) => economic_issuance_digest(&v),
            Self::Burn(v) => economic_burn_digest(&v),
            Self::Reserve(v) => economic_reserve_digest(&v),
        }
    }
    pub fn bytes(self) -> Vec<u8> {
        match self {
            Self::Issue(v) => issuance_bytes(&v),
            Self::Burn(v) => burn_bytes(&v),
            Self::Reserve(v) => reserve_bytes(&v),
        }
    }
    pub fn permit(self, policy: &SuppliesPolicy) -> Result<()> {
        match self {
            Self::Issue(v) => policy.permit(&v)?,
            Self::Burn(v) => policy.permit_burn(&v)?,
            Self::Reserve(v) => {
                policy.validate()?;
                if v.universe != policy.universe || v.history != policy.history {
                    return Err(SuppliesRejection::Scope.into());
                }
                if v.policy != policy_digest(policy)? {
                    return Err(SuppliesRejection::Policy.into());
                }
                if v.owner != v.actor {
                    return Err(SuppliesRejection::Unauthorized.into());
                }
                if v.content != SUPPLIES_CONTENT {
                    return Err(SuppliesRejection::UnsupportedContent.into());
                }
                if v.amount == 0 {
                    return Err(SuppliesRejection::Limit.into());
                }
            }
        }
        Ok(())
    }
    pub fn apply(self, balance: &mut Balances) -> Result<()> {
        match self {
            Self::Issue(v) => mint(balance, v.amount),
            Self::Burn(v) => destroy(balance, v.amount),
            Self::Reserve(v) => reserve_units(balance, v.amount),
        }
    }
}
