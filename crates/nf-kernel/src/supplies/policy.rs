use super::*;
impl SuppliesPolicy {
    pub fn validate(&self) -> Result<(), SuppliesRejection> {
        if self.issuers.is_empty()
            || self
                .issuers
                .len()
                .checked_add(self.burners.len())
                .ok_or(SuppliesRejection::Limit)?
                > MAX_ISSUER_RULES
        {
            return Err(SuppliesRejection::Limit);
        }
        for (index, rule) in self.issuers.iter().enumerate() {
            if rule.content != SUPPLIES_CONTENT {
                return Err(SuppliesRejection::UnsupportedContent);
            }
            if rule.maximum == 0 || rule.origin.lineage == [0; 32] {
                return Err(SuppliesRejection::Policy);
            }
            if self.issuers[..index].iter().any(|prior| {
                prior.issuer == rule.issuer
                    && prior.content == rule.content
                    && prior.origin == rule.origin
                    && prior.reason == rule.reason
            }) {
                return Err(SuppliesRejection::Policy);
            }
        }
        for (index, rule) in self.burners.iter().enumerate() {
            if rule.content != SUPPLIES_CONTENT {
                return Err(SuppliesRejection::UnsupportedContent);
            }
            if rule.maximum == 0 || rule.origin.lineage == [0; 32] {
                return Err(SuppliesRejection::Policy);
            }
            if self.burners[..index].iter().any(|prior| {
                prior.issuer == rule.issuer
                    && prior.content == rule.content
                    && prior.origin == rule.origin
                    && prior.reason == rule.reason
            }) {
                return Err(SuppliesRejection::Policy);
            }
        }
        Ok(())
    }
    pub fn permit(&self, issuance: &Issuance) -> Result<(), SuppliesRejection> {
        self.validate()?;
        if issuance.universe != self.universe || issuance.history != self.history {
            return Err(SuppliesRejection::Scope);
        }
        if issuance.policy != policy_digest(self)? {
            return Err(SuppliesRejection::Policy);
        }
        if issuance.content != SUPPLIES_CONTENT {
            return Err(SuppliesRejection::UnsupportedContent);
        }
        let rule = self
            .issuers
            .iter()
            .find(|rule| {
                rule.issuer == issuance.actor
                    && rule.content == issuance.content
                    && rule.origin == issuance.origin
                    && rule.reason == issuance.reason
            })
            .ok_or(SuppliesRejection::Unauthorized)?;
        if issuance.amount == 0 || issuance.amount > rule.maximum {
            return Err(SuppliesRejection::Limit);
        }
        Ok(())
    }
    pub fn permit_burn(&self, burn: &Burn) -> Result<(), SuppliesRejection> {
        self.validate()?;
        if burn.universe != self.universe || burn.history != self.history {
            return Err(SuppliesRejection::Scope);
        }
        if burn.policy != policy_digest(self)? {
            return Err(SuppliesRejection::Policy);
        }
        if burn.content != SUPPLIES_CONTENT {
            return Err(SuppliesRejection::UnsupportedContent);
        }
        let rule = self
            .burners
            .iter()
            .find(|rule| {
                rule.issuer == burn.actor
                    && rule.content == burn.content
                    && rule.origin == burn.origin
                    && rule.reason == burn.reason
            })
            .ok_or(SuppliesRejection::Unauthorized)?;
        if burn.amount == 0 || burn.amount > rule.maximum {
            return Err(SuppliesRejection::Limit);
        }
        Ok(())
    }
}
