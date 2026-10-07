use crate::{
    ShadowAuthority, ShadowEvaluation, ShadowInput, ShadowMetadata, ShadowOutput, Unavailable,
    WarOperation, WarResult, WorldCommitment, commit_world,
};
use nf_contract::identity::EntityId;
use nf_nex_boundary::NexWorld;
use sha2::{Digest, Sha256};
/// Immutable full-declared-world diagnostic, with no public constructor or scope conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldShadowEvaluation {
    world: WorldCommitment,
    copied: ShadowEvaluation,
    method: WarOperation,
    target: Option<EntityId>,
    binding: [u8; 32],
}
impl WorldShadowEvaluation {
    pub fn world_commitment(&self) -> WorldCommitment {
        self.world
    }
    pub fn binding_digest(&self) -> [u8; 32] {
        self.binding
    }
    pub fn copied_input_digest(&self) -> [u8; 32] {
        self.copied.input_digest()
    }
    pub fn output(&self) -> &WarResult {
        match self.copied.output() {
            ShadowOutput::War(v) => v,
            ShadowOutput::SelectedPeace(_) => unreachable!("private war-only construction"),
        }
    }
    pub fn authority(&self) -> ShadowAuthority {
        ShadowAuthority::ShadowOnly
    }
}
/// Owner-provided immutable values fence; missed native mutations cannot be detected by hashing.
pub struct WorldShadowSession {
    metadata: ShadowMetadata,
    world: WorldCommitment,
    active: bool,
}
impl WorldShadowSession {
    pub fn new(metadata: ShadowMetadata, world: &NexWorld) -> Result<Self, Unavailable> {
        let commitment = commit_world(world)?;
        // Validate named-instance facts without imposing the unrelated generation policy.
        bound_input(
            &metadata,
            world,
            if metadata.concern_instance.is_some() {
                WarOperation::IsValid
            } else {
                WarOperation::Generate
            },
            metadata.concern_instance,
        )?;
        Ok(Self {
            metadata,
            world: commitment,
            active: true,
        })
    }
    pub fn invalidate(&mut self) {
        self.active = false;
    }
    pub fn evaluate_war(
        &mut self,
        world: &NexWorld,
        method: WarOperation,
        target: Option<EntityId>,
    ) -> Result<WorldShadowEvaluation, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let result = self.evaluate_current(world, method, target);
        if result.is_err() {
            self.active = false;
        }
        result
    }
    fn evaluate_current(
        &self,
        world: &NexWorld,
        method: WarOperation,
        target: Option<EntityId>,
    ) -> Result<WorldShadowEvaluation, Unavailable> {
        let commitment = commit_world(world)?;
        if commitment != self.world {
            return Err(Unavailable::Stale);
        }
        let input = bound_input(&self.metadata, world, method, target)?;
        let copied = crate::evaluate_shadow(&self.metadata, &input)?;
        let binding = binding(commitment, copied.input_digest(), method, target);
        Ok(WorldShadowEvaluation {
            world: commitment,
            copied,
            method,
            target,
            binding,
        })
    }
    pub fn accept_war<'a>(
        &self,
        result: &'a WorldShadowEvaluation,
        current_metadata: &ShadowMetadata,
        current_world: &NexWorld,
        current_method: WarOperation,
        current_target: Option<EntityId>,
    ) -> Result<&'a WarResult, Unavailable> {
        if !self.active {
            return Err(Unavailable::Disabled);
        }
        let current_world_digest = commit_world(current_world)?;
        let input = bound_input(
            current_metadata,
            current_world,
            current_method,
            current_target,
        )?;
        let copied_digest = crate::input_digest(current_metadata, &input)?;
        let current_binding = binding(
            current_world_digest,
            copied_digest,
            current_method,
            current_target,
        );
        if self.metadata != *current_metadata
            || self.metadata != *result.copied.metadata()
            || self.world != result.world
            || self.world != current_world_digest
            || result.method != current_method
            || result.target != current_target
            || result.copied.input_digest() != copied_digest
            || result.binding != current_binding
        {
            return Err(Unavailable::Stale);
        }
        Ok(result.output())
    }
}
fn bound_input(
    m: &ShadowMetadata,
    w: &NexWorld,
    method: WarOperation,
    target: Option<EntityId>,
) -> Result<ShadowInput, Unavailable> {
    match (method, target) {
        (WarOperation::Generate, Some(_)) => return Err(Unavailable::Unsupported),
        (WarOperation::Update | WarOperation::IsValid, None) => {
            return Err(Unavailable::MissingFact);
        }
        (WarOperation::Generate, None)
        | (WarOperation::Update | WarOperation::IsValid, Some(_)) => {}
    }
    let input = ShadowInput::War {
        operation: method,
        facts: crate::war_from_world(w, method, target)?,
    };
    crate::input_digest(m, &input)?;
    if m.provenance != w.snapshot().provenance
        || m.subject_faction != w.snapshot().subject_faction
        || m.concern_instance != target
    {
        return Err(Unavailable::Stale);
    }
    Ok(input)
}
fn binding(
    w: WorldCommitment,
    copied: [u8; 32],
    method: WarOperation,
    target: Option<EntityId>,
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NF-NEX-WORLD-EVAL-1\0");
    hash.update(1_u16.to_le_bytes());
    hash.update(w.digest());
    hash.update(copied);
    hash.update([match method {
        WarOperation::Generate => 1,
        WarOperation::Update => 2,
        WarOperation::IsValid => 3,
    }]);
    hash.update([u8::from(target.is_some())]);
    if let Some(t) = target {
        hash.update(t.as_bytes());
    }
    hash.finalize().into()
}
