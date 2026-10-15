use super::{HostError, PortableAdmissionInputV1};
use nf_nex_shadow::{ShadowInput, ShadowMetadata};

// Size checks precede maintained validators, allocation and retained clones.
// Canonical text, identity, scalar and nested validity still belong to the encoder.
pub(super) fn metadata(value: &ShadowMetadata) -> Result<(), HostError> {
    if value.provenance.source_commit.len() != 40
        || value.subject_faction.is_empty()
        || value.subject_faction.len() > 128
    {
        return Err(HostError::InvalidRequest);
    }
    Ok(())
}
pub(super) fn copied(value: &ShadowInput) -> Result<(), HostError> {
    let valid = match value {
        ShadowInput::War { facts, .. } => {
            let p = &facts.priority;
            facts.existing.len() <= 256
                && p.traits.len() <= 64
                && p.tags.len() == 5
                && p.existing.len() <= 64
                && p.traits.iter().chain(&p.tags).all(|s| s.len() <= 256)
                && p.existing.iter().all(|m| m.id.len() <= 128)
        }
        ShadowInput::SelectedPeace(facts) => {
            facts.faction.len() <= 128
                && facts.enemy.len() <= 128
                && facts.draws.len() <= 2
                && facts.draws.iter().all(|d| d.purpose.len() <= 256)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(HostError::InvalidRequest)
    }
}
pub(super) fn canonical(
    value: &ShadowMetadata,
    first: PortableAdmissionInputV1<'_>,
) -> Result<(), HostError> {
    metadata(value)?;
    let bytes = match first {
        PortableAdmissionInputV1::Copied(input) => {
            copied(input)?;
            nf_nex_shadow::encode_input(value, input)
        }
        PortableAdmissionInputV1::Action(input) => nf_nex_shadow::encode_action_input(value, input),
    }
    .map_err(|_| HostError::InvalidRequest)?;
    if bytes.len() > 65_536 {
        return Err(HostError::Bounds);
    }
    // Discard canonical private data before retaining a session or returning admission.
    drop(bytes);
    Ok(())
}
