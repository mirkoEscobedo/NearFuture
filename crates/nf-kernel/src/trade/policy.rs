use super::*;
use crate::supplies::{SUPPLIES_CONTENT, TrustClass};
impl TradePolicy {
    pub fn validate(&self) -> Result<(), TradeRejection> {
        self.supplies
            .validate()
            .map_err(|_| TradeRejection::Policy)?;
        if self.allowed_origins.is_empty() || self.allowed_origins.len() > MAX_ALLOWED_ORIGINS {
            return Err(TradeRejection::Limit);
        }
        for (index, origin) in self.allowed_origins.iter().enumerate() {
            if origin.trust != TrustClass::Canonical
                || origin.lineage == [0; 32]
                || self.allowed_origins[..index].contains(origin)
                || !self
                    .supplies
                    .issuers
                    .iter()
                    .any(|rule| rule.origin == *origin)
            {
                return Err(TradeRejection::Policy);
            }
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<[u8; 32], TradeRejection> {
        trade_policy_digest(self)
    }
    pub fn permit_cancel(&self, cancel: &CancelOffer) -> Result<(), TradeRejection> {
        self.validate()?;
        if cancel.universe != self.supplies.universe || cancel.history != self.supplies.history {
            return Err(TradeRejection::Scope);
        }
        if cancel.policy != self.digest()? {
            return Err(TradeRejection::Policy);
        }
        if cancel.version == 0 {
            return Err(TradeRejection::Limit);
        }
        match self.cancel_rule {
            CancelRule::EitherNamedParty => Ok(()),
        }
    }
    pub fn permit_offer(&self, offer: &OfferTerms) -> Result<(), TradeRejection> {
        self.validate()?;
        if offer.universe != self.supplies.universe || offer.history != self.supplies.history {
            return Err(TradeRejection::Scope);
        }
        if offer.policy != self.digest()? {
            return Err(TradeRejection::Policy);
        }
        if offer.maker == offer.taker {
            return Err(TradeRejection::Unauthorized);
        }
        if offer.version == 0 || offer.expires_at == 0 {
            return Err(TradeRejection::Limit);
        }
        for asset in [offer.give, offer.want] {
            if asset.content != SUPPLIES_CONTENT {
                return Err(TradeRejection::UnsupportedContent);
            }
            if !self.allowed_origins.contains(&asset.origin) {
                return Err(TradeRejection::Policy);
            }
            if asset.amount == 0 {
                return Err(TradeRejection::Limit);
            }
        }
        Ok(())
    }
}
