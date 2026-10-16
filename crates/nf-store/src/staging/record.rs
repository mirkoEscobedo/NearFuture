use super::{ConfigOnlyTransition, StageBinding, StageError};
use crate::{KnownFrontiers, migration::staging_record::Reader};
use nf_contract::identity::{EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;
const DOMAIN: &[u8] = b"NF-MIGRATION-STAGE-1";
pub(crate) const MAX_BYTES: usize = 4096;

pub(crate) fn encode(plan: &ConfigOnlyTransition) -> Result<Vec<u8>, StageError> {
    let binding = plan.binding;
    let mut bytes = DOMAIN.to_vec();
    bytes.extend_from_slice(binding.known.scope.universe.as_bytes());
    bytes.extend_from_slice(binding.known.scope.history.as_bytes());
    bytes.extend_from_slice(&binding.known.store_revision.to_be_bytes());
    bytes.extend_from_slice(&binding.known.event_sequence.0.to_be_bytes());
    match binding.known.membership_revision {
        Some(revision) => {
            bytes.push(1);
            bytes.extend_from_slice(&revision.to_be_bytes());
        }
        None => bytes.push(0),
    }
    bytes.extend_from_slice(&binding.head);
    bytes.extend_from_slice(&binding.snapshot);
    match binding.membership {
        Some(digest) => {
            bytes.push(1);
            bytes.extend_from_slice(&digest);
        }
        None => bytes.push(0),
    }
    for value in [&plan.from, &plan.to] {
        bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(value);
    }
    if bytes.len() > MAX_BYTES {
        return Err(StageError::Storage(crate::StoreError::Limit));
    }
    Ok(bytes)
}
fn flag(input: &mut Reader<'_>) -> Result<bool, StageError> {
    match input.fixed::<1>()?[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StageError::Corrupt),
    }
}
fn descriptor(input: &mut Reader<'_>) -> Result<Vec<u8>, StageError> {
    let size = input.u32()? as usize;
    if size > crate::migration::staging_record::MAX_BYTES {
        return Err(StageError::Corrupt);
    }
    let end = input.at.checked_add(size).ok_or(StageError::Corrupt)?;
    let value = input.bytes.get(input.at..end).ok_or(StageError::Corrupt)?;
    input.at = end;
    crate::migration::staging_record::decode(value)?;
    Ok(value.to_vec())
}
pub(crate) fn decode(bytes: &[u8]) -> Result<ConfigOnlyTransition, StageError> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(DOMAIN) {
        return Err(StageError::Corrupt);
    }
    let mut input = Reader {
        bytes,
        at: DOMAIN.len(),
    };
    let scope = Scope {
        universe: UniverseId::from_bytes(input.fixed()?),
        history: HistoryId::from_bytes(input.fixed()?),
    };
    let store_revision = u64::from_be_bytes(input.fixed()?);
    let event_sequence = EventSeq(u64::from_be_bytes(input.fixed()?));
    let membership_revision = if flag(&mut input)? {
        Some(u64::from_be_bytes(input.fixed()?))
    } else {
        None
    };
    let head = input.fixed()?;
    let snapshot = input.fixed()?;
    let membership = if flag(&mut input)? {
        Some(input.fixed()?)
    } else {
        None
    };
    let plan = ConfigOnlyTransition {
        binding: StageBinding {
            known: KnownFrontiers {
                scope,
                event_sequence,
                store_revision,
                membership_revision,
            },
            head,
            snapshot,
            membership,
        },
        from: descriptor(&mut input)?,
        to: descriptor(&mut input)?,
    };
    if input.at != bytes.len() || encode(&plan)? != bytes {
        return Err(StageError::Corrupt);
    }
    Ok(plan)
}
