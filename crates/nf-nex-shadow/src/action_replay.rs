use crate::{
    MakePeaceEligibility, ShadowEvaluation, ShadowMetadata, ShadowSession, Unavailable,
    encode_action_input,
};
use alloc::string::String;
use nf_contract::identity::{
    BranchId, CampaignId, EntityId, EventSeq, HistoryId, ProviderId, RuntimeSession, UniverseId,
};
use nf_nex_boundary::{Observation, Provenance};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedActionInput {
    pub metadata: ShadowMetadata,
    pub facts: MakePeaceEligibility,
}

/// Decode only the closed private action V2; never a native/world or authority certificate.
pub fn decode_action_input(bytes: &[u8]) -> Result<DecodedActionInput, Unavailable> {
    if bytes.len() > 571 {
        return Err(Unavailable::Limit);
    }
    if bytes.len() < 18 || !bytes.starts_with(b"NF-NEX-SHADOW-2\0") || bytes[16..18] != [2, 0] {
        return Err(Unavailable::Unsupported);
    }
    let mut cursor = Cursor {
        bytes,
        position: 18,
    };
    let source_commit = cursor.text(40)?;
    let source_digest = cursor.array()?;
    let runtime_digest = cursor.array()?;
    let ruleset_digest = cursor.array()?;
    let merged_config_digest = cursor.array()?;
    let reference_digest = cursor.array()?;
    let corpus_digest = cursor.array()?;
    let implementation_digest = cursor.array()?;
    let capture_policy_digest = cursor.array()?;
    let universe = UniverseId::from_bytes(cursor.array()?);
    let history = HistoryId::from_bytes(cursor.array()?);
    let provider = ProviderId::from_bytes(cursor.array()?);
    let campaign = CampaignId::from_bytes(cursor.array()?);
    let branch = BranchId::from_bytes(cursor.array()?);
    let subject_faction = cursor.text(128)?;
    let concern_instance = if cursor.boolean()? {
        Some(EntityId::from_bytes(cursor.array()?))
    } else {
        None
    };
    let runtime_session = RuntimeSession(cursor.u64()?);
    let frontier = EventSeq(cursor.u64()?);
    let observation = match cursor.byte()? {
        1 => Observation::Synthetic,
        2 => Observation::CapturedUnverified,
        _ => return Err(Unavailable::Unsupported),
    };
    if cursor.byte()? != 1 || cursor.byte()? != 1 {
        return Err(Unavailable::Unsupported);
    }
    let diplomacy_enabled = cursor.boolean()?;
    let concern_can_make_peace = cursor.boolean()?;
    let target_hostile = if cursor.boolean()? {
        Some(cursor.boolean()?)
    } else {
        None
    };
    let faction_diplomacy_disabled = cursor.boolean()?;
    if cursor.position != bytes.len() {
        return Err(Unavailable::Unsupported);
    }
    let metadata = ShadowMetadata {
        provenance: Provenance {
            source_commit,
            source_digest,
            runtime_digest,
            ruleset_digest,
            merged_config_digest,
            universe,
            history,
            runtime_session,
            frontier,
            observation,
        },
        subject_faction,
        concern_instance,
        provider,
        campaign,
        branch,
        reference_digest,
        corpus_digest,
        implementation_digest,
        capture_policy_digest,
    };
    let facts = MakePeaceEligibility {
        diplomacy_enabled,
        concern_can_make_peace,
        target_hostile,
        faction_diplomacy_disabled,
    };
    if encode_action_input(&metadata, &facts)? != bytes {
        return Err(Unavailable::Unsupported);
    }
    Ok(DecodedActionInput { metadata, facts })
}

/// Explicit caller-reviewed synthetic reproduction only; no IO or native context acceptance.
pub fn replay_synthetic_action(bytes: &[u8]) -> Result<ShadowEvaluation, Unavailable> {
    let decoded = decode_action_input(bytes)?;
    if decoded.metadata.provenance.observation != nf_nex_boundary::Observation::Synthetic {
        return Err(Unavailable::Unsupported);
    }
    ShadowSession::new(decoded.metadata).evaluate_action(&decoded.facts)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], Unavailable> {
        let end = self.position.checked_add(count).ok_or(Unavailable::Limit)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(Unavailable::Unsupported)?;
        self.position = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Unavailable> {
        self.take(N)?
            .try_into()
            .map_err(|_| Unavailable::Unsupported)
    }

    fn byte(&mut self) -> Result<u8, Unavailable> {
        Ok(self.array::<1>()?[0])
    }

    fn boolean(&mut self) -> Result<bool, Unavailable> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Unavailable::Unsupported),
        }
    }

    fn u64(&mut self) -> Result<u64, Unavailable> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn text(&mut self, limit: usize) -> Result<String, Unavailable> {
        let length =
            usize::try_from(u32::from_le_bytes(self.array()?)).map_err(|_| Unavailable::Limit)?;
        if length == 0 || length > limit {
            return Err(Unavailable::Limit);
        }
        let value =
            core::str::from_utf8(self.take(length)?).map_err(|_| Unavailable::Unsupported)?;
        crate::validation::text(value)?;
        Ok(String::from(value))
    }
}
