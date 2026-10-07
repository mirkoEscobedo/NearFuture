use super::*;
use alloc::collections::BTreeSet;
impl MiniatureWorld {
    pub(crate) fn validate(&self) -> MiniatureResult<()> {
        let m = self.metadata;
        if m.aggregate == m.provider_aggregate {
            return Err(MiniatureRejection::DuplicateIdentity);
        }
        if m.event_sequence.0 < m.tick.0
            || m.provider_revision.0 != m.tick.0
            || self.component.revision().0 > m.tick.0.saturating_mul(33)
        {
            return Err(MiniatureRejection::InvalidValue);
        }
        nf_world::validate_at(&self.component, m.tick)?;
        if self.outcomes.len() > MAX_MINIATURE_OUTCOMES {
            return Err(MiniatureRejection::Limit);
        }
        if m.event_sequence.0 == 0 && !self.outcomes.is_empty() {
            return Err(MiniatureRejection::InvalidValue);
        }
        if self
            .outcomes
            .windows(2)
            .any(|w| (w[0].operation, w[0].job) >= (w[1].operation, w[1].job))
        {
            return Err(MiniatureRejection::InvalidValue);
        }
        let mut operations = BTreeSet::new();
        let mut jobs = BTreeSet::new();
        if self
            .outcomes
            .iter()
            .any(|v| !operations.insert(v.operation) || !jobs.insert(v.job))
        {
            return Err(MiniatureRejection::DuplicateIdentity);
        }
        Ok(())
    }
}
