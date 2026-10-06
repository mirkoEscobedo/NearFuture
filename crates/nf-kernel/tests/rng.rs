use nf_contract::identity::*;
use nf_kernel::*;
#[test]
fn scoped_rng_has_fixed_vector_and_no_authority_or_session_inputs() {
    let scope = RngScope {
        seed: [3; 32],
        history: HistoryId::from_bytes([2; 16]),
        ruleset_hash: [4; 32],
        provider: ProviderId::from_bytes([9; 16]),
        entity: EntityId::from_bytes([10; 16]),
        tick: WorldTick(0),
        operation: OperationId::from_bytes([11; 16]),
        draw: 0,
    };
    assert_eq!(scoped_draw(&scope), 15213394994864267995); // independent Node SHA-256 fixture
}
