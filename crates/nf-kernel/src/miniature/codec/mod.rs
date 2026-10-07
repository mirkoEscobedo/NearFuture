use super::*;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
mod primitives;
use primitives::*;
mod actions;
mod batch;
mod events;
mod frontier;
mod holds;
mod intent;
mod snapshot;
pub fn miniature_state_hash(world: &MiniatureWorld) -> MiniatureResult<[u8; 32]> {
    Ok(Sha256::digest(encode_miniature_snapshot(world)?).into())
}
pub fn encode_miniature_snapshot(world: &MiniatureWorld) -> MiniatureResult<Vec<u8>> {
    snapshot::encode_in(world, &mut Budget::default())
}
pub fn decode_miniature_snapshot(bytes: &[u8]) -> MiniatureResult<MiniatureWorld> {
    snapshot::decode_in(bytes, &mut Budget::default())
}
fn rejection(code: u8) -> MiniatureResult<Option<MiniatureRejection>> {
    use MiniatureRejection::*;
    let tags = [
        InvalidReference,
        DuplicateIdentity,
        Limit,
        InvalidValue,
        UnsupportedProvider,
        Unauthorized,
        StaleRevision,
        Conflict,
        Overflow,
        InvalidProposal,
        ProviderFailed,
        StaleSession,
        FencedAuthority,
        UnknownJob,
        DuplicateJob,
        InvalidFrontier,
        StateMismatch,
        Resources,
        Locked,
        AdmissionChanged,
        Cancelled,
    ];
    if code == 0 {
        Ok(None)
    } else {
        tags.get(usize::from(code) - 1)
            .copied()
            .map(Some)
            .ok_or(InvalidValue)
    }
}
pub fn encode_miniature_intent(intent: &MiniatureIntent) -> MiniatureResult<Vec<u8>> {
    intent::encode_in(intent, &mut Budget::default())
}
pub fn decode_miniature_intent(bytes: &[u8]) -> MiniatureResult<MiniatureIntent> {
    intent::decode_in(bytes, &mut Budget::default())
}
pub fn miniature_request_binding(
    intent: &MiniatureIntent,
) -> MiniatureResult<nf_contract::canonical::binding::RequestBinding> {
    Ok(nf_contract::canonical::binding::RequestBinding {
        request_id: intent.request,
        account_id: intent.actor,
        device_id: intent.device,
        universe_id: intent.universe,
        history_id: intent.history,
        operation_kind: 4,
        payload_digest: Sha256::digest(encode_miniature_intent(intent)?).into(),
    })
}
pub fn encode_miniature_frontier(frontier: &MiniatureFrontier) -> MiniatureResult<Vec<u8>> {
    frontier::encode_in(frontier, &mut Budget::default())
}
pub fn decode_miniature_frontier(bytes: &[u8]) -> MiniatureResult<MiniatureFrontier> {
    frontier::decode_in(bytes, &mut Budget::default())
}
pub fn encode_miniature_batch(batch: &MiniatureBatch) -> MiniatureResult<Vec<u8>> {
    batch::encode_in(batch, &mut Budget::default())
}
pub fn decode_miniature_batch(
    bytes: &[u8],
    before: &MiniatureWorld,
) -> MiniatureResult<MiniatureBatch> {
    batch::decode_in(bytes, before, &mut Budget::default())
}
