mod driver_support;
use nf_contract::identity::*;
use nf_store::miniature::MiniatureRequestStatus;
use nf_world_driver::{ActionRequest, ActionSelection, Driver};
#[test]
fn real_current_activity_and_owner_fence_commit_action_once_then_empty_tick_without_duplicate_retry()
 {
    let fixture = driver_support::Fixture::new();
    let mut driver = Driver::create(fixture.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let request = ActionRequest {
        request: RequestId::from_bytes([40; 16]),
        operation: OperationId::from_bytes([41; 16]),
        job: JobId::from_bytes([42; 16]),
        action: ActionSelection::Colony {
            site: 1,
            faction: 0,
        },
    };
    let pending = driver.prepare_action(&request).unwrap();
    let before = driver.status().unwrap();
    assert!(driver.advance_pending(false).is_err());
    assert_eq!(driver.status().unwrap(), before);
    driver
        .advance_pending(true)
        .expect("actual signed current-N activity plus fresh guarded owner commit");
    let committed = driver.status().unwrap();
    assert_eq!(committed.world.metadata().tick, WorldTick(1));
    assert_eq!(committed.world.metadata().event_sequence, EventSeq(1));
    assert_eq!(
        committed.world.metadata().provider_revision,
        AggregateRevision(1)
    );
    assert!(committed.pending.is_none());
    let faction = committed
        .world
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 0)
        .unwrap();
    assert_eq!((faction.credits, faction.supplies), (100, 60));
    let retry = driver.prepare_action(&request).unwrap();
    assert_eq!(retry.intent, pending.intent);
    assert_eq!(retry.binding_digest, pending.binding_digest);
    assert_eq!(
        retry.status,
        MiniatureRequestStatus::Committed {
            operation: request.operation,
            sequence: EventSeq(1),
            rejection: None
        }
    );
    assert_eq!(driver.status().unwrap(), committed);
    driver.advance_empty(true).unwrap();
    let empty = driver.status().unwrap();
    assert_eq!(empty.world.metadata().tick, WorldTick(2));
    assert_eq!(
        empty.world.metadata().provider_revision,
        AggregateRevision(2)
    );
    assert_eq!(empty.world.component(), committed.world.component());
}
