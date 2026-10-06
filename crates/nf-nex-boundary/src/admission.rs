use crate::{BoundaryError, Snapshot};
use alloc::{string::String, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Authority {
    ShadowOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCertification {
    Unobserved,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompatibilityFinding {
    LegacyOnlyDefinition { id: String, class_path: String },
    UncharacterizedListener { class_path: String, origin: String },
    UncharacterizedDirectCall { class_path: String, origin: String },
    IncompleteInventory,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub authority: Authority,
    pub runtime: RuntimeCertification,
    pub findings: Vec<CompatibilityFinding>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NexWorld {
    snapshot: Snapshot,
    assessment: Assessment,
}
impl NexWorld {
    pub fn admit(snapshot: Snapshot) -> Result<Self, BoundaryError> {
        crate::validation::validate(&snapshot)?;
        let assessment = assess_extensions(&snapshot.extensions)?;
        if !assessment.findings.is_empty() {
            return Err(BoundaryError::InvalidFact("extension inventory"));
        }
        Ok(Self {
            snapshot,
            assessment,
        })
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn assessment(&self) -> &Assessment {
        &self.assessment
    }
}

pub fn assess_extensions(
    inventory: &crate::ExtensionInventory,
) -> Result<Assessment, BoundaryError> {
    let mut budget = crate::validation::Budget::default();
    budget.entries(inventory.listeners.len(), 256)?;
    budget.entries(inventory.direct_calls.len(), 256)?;
    budget.entries(inventory.other_definitions.len(), 256)?;
    for registration in inventory.listeners.iter().chain(&inventory.direct_calls) {
        budget.text(&registration.class_path)?;
        budget.text(&registration.origin)?;
    }
    for definition in &inventory.other_definitions {
        budget.text(&definition.id)?;
        budget.text(&definition.class_path)?;
    }
    let mut findings = Vec::new();
    if !inventory.complete {
        findings.push(CompatibilityFinding::IncompleteInventory);
    }
    for listener in &inventory.listeners {
        findings.push(CompatibilityFinding::UncharacterizedListener {
            class_path: listener.class_path.clone(),
            origin: listener.origin.clone(),
        });
    }
    for call in &inventory.direct_calls {
        findings.push(CompatibilityFinding::UncharacterizedDirectCall {
            class_path: call.class_path.clone(),
            origin: call.origin.clone(),
        });
    }
    for definition in &inventory.other_definitions {
        findings.push(CompatibilityFinding::LegacyOnlyDefinition {
            id: definition.id.clone(),
            class_path: definition.class_path.clone(),
        });
    }
    Ok(Assessment {
        authority: Authority::ShadowOnly,
        runtime: RuntimeCertification::Unobserved,
        findings,
    })
}
