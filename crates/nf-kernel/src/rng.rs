use nf_contract::identity::*;
use sha2::{Digest, Sha256};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RngScope {
    pub seed: [u8; 32],
    pub history: HistoryId,
    pub ruleset_hash: [u8; 32],
    pub provider: ProviderId,
    pub entity: EntityId,
    pub tick: WorldTick,
    pub operation: OperationId,
    pub draw: u64,
}
/// SHA-256 first eight bytes interpreted LE. No ambient entropy, leader term or session.
pub fn scoped_draw(scope: &RngScope) -> u64 {
    let mut hash = Sha256::new();
    hash.update(b"NF-RNG-1\0");
    hash.update(scope.seed);
    hash.update(scope.history.as_bytes());
    hash.update(scope.ruleset_hash);
    hash.update(scope.provider.as_bytes());
    hash.update(scope.entity.as_bytes());
    hash.update(scope.tick.0.to_le_bytes());
    hash.update(scope.operation.as_bytes());
    hash.update(scope.draw.to_le_bytes());
    let digest = hash.finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(bytes)
}
