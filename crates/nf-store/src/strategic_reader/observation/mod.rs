//! Closed redacted observation values. Only the admitted reader constructs them.
mod counts;
mod providers;
use crate::{
    KnownFrontiers,
    error::{Result, StoreError},
    model::State,
};
pub use counts::StrategicSupportCountsV1;
pub use providers::{StrategicSupportAssignmentV1, StrategicSupportManifestV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportProtection {
    Checked,
    NotSupplied,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategicSupportImplementation {
    RegisteredUnverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategicSupportRulesetBinding {
    MatchesPersistedRuleset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategicSupportUnavailable {
    CurrentOwner,
    AuthorityTerm,
    Selector,
    ProviderLag,
    Taint,
    NativeTransfer,
    TransportReceipt,
    RuntimeProvenance,
}

const UNAVAILABLE: [StrategicSupportUnavailable; 8] = [
    StrategicSupportUnavailable::CurrentOwner,
    StrategicSupportUnavailable::AuthorityTerm,
    StrategicSupportUnavailable::Selector,
    StrategicSupportUnavailable::ProviderLag,
    StrategicSupportUnavailable::Taint,
    StrategicSupportUnavailable::NativeTransfer,
    StrategicSupportUnavailable::TransportReceipt,
    StrategicSupportUnavailable::RuntimeProvenance,
];

#[derive(Clone, Debug)]
pub struct StrategicSupportProtectionV1 {
    membership_revision: SupportProtection,
}

impl StrategicSupportProtectionV1 {
    pub fn scope(&self) -> SupportProtection {
        SupportProtection::Checked
    }
    pub fn event(&self) -> SupportProtection {
        SupportProtection::Checked
    }
    pub fn store_revision(&self) -> SupportProtection {
        SupportProtection::Checked
    }
    pub fn membership_revision(&self) -> SupportProtection {
        self.membership_revision
    }
    pub fn authority_term(&self) -> SupportProtection {
        SupportProtection::NotApplicable
    }
}

#[derive(Clone, Debug)]
pub struct StrategicSupportObservationV1 {
    event_sequence: u64,
    store_revision: u64,
    membership_revision: Option<u64>,
    protected: StrategicSupportProtectionV1,
    counts: StrategicSupportCountsV1,
    manifests: Vec<StrategicSupportManifestV1>,
    assignments: Vec<StrategicSupportAssignmentV1>,
}

impl StrategicSupportObservationV1 {
    pub(super) fn from_recovered(
        state: &State,
        actual: KnownFrontiers,
        protected: KnownFrontiers,
        members: Option<u64>,
    ) -> Result<Self> {
        let spec = state.world.to_spec();
        let counts = StrategicSupportCountsV1::from_recovered(state, &spec, members)?;
        let (manifests, assignments) = providers::from_recovered(spec)?;
        Ok(Self {
            event_sequence: actual.event_sequence.0,
            store_revision: actual.store_revision,
            membership_revision: actual.membership_revision,
            protected: StrategicSupportProtectionV1 {
                membership_revision: if protected.membership_revision.is_some() {
                    SupportProtection::Checked
                } else {
                    SupportProtection::NotSupplied
                },
            },
            counts,
            manifests,
            assignments,
        })
    }
    pub fn profile(&self) -> u32 {
        1
    }
    pub fn schema_version(&self) -> u32 {
        1
    }
    pub fn event_sequence(&self) -> u64 {
        self.event_sequence
    }
    pub fn store_revision(&self) -> u64 {
        self.store_revision
    }
    pub fn membership_revision(&self) -> Option<u64> {
        self.membership_revision
    }
    pub fn protected(&self) -> &StrategicSupportProtectionV1 {
        &self.protected
    }
    pub fn counts(&self) -> &StrategicSupportCountsV1 {
        &self.counts
    }
    pub fn manifests(&self) -> &[StrategicSupportManifestV1] {
        &self.manifests
    }
    pub fn assignments(&self) -> &[StrategicSupportAssignmentV1] {
        &self.assignments
    }
    pub fn unavailable(&self) -> &[StrategicSupportUnavailable] {
        &UNAVAILABLE
    }
}

pub(super) fn count(length: usize) -> Result<u64> {
    u64::try_from(length).map_err(|_| StoreError::Limit)
}
