mod driver_support;
use nf_contract::identity::*;
use nf_store::miniature::MiniatureRequestStatus;
use nf_world_driver::{ActionRequest, ActionSelection, Driver};
#[test]
fn signed_action_preparation_derives_hold_and_retry_preserves_original_intent_and_revisions() {
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
    let first = driver
        .prepare_action(&request)
        .expect("fresh real economic proof prepares typed pending action");
    assert_eq!(
        first.status,
        MiniatureRequestStatus::Pending {
            operation: request.operation
        }
    );
    assert_eq!(
        first.intent.expected[&fixture.options.genesis.aggregate],
        AggregateRevision(0)
    );
    let snapshot = driver.status().unwrap();
    let hold = snapshot.pending.as_ref().unwrap().reservation().unwrap();
    assert_eq!((hold.credits(), hold.supplies()), (100, 40));
    assert_eq!(snapshot.world.metadata().tick, WorldTick(0));
    assert_eq!(
        snapshot
            .world
            .component()
            .factions()
            .iter()
            .find(|f| f.ordinal == 0)
            .unwrap()
            .credits,
        200
    );
    assert_eq!(driver.prepare_action(&request).unwrap(), first);
    assert_eq!(driver.status().unwrap(), snapshot);
    let mut changed = request.clone();
    changed.action = ActionSelection::Colony {
        site: 3,
        faction: 0,
    };
    assert!(driver.prepare_action(&changed).is_err());
    changed = request;
    changed.operation = OperationId::from_bytes([43; 16]);
    assert!(driver.prepare_action(&changed).is_err());
    assert_eq!(driver.status().unwrap(), snapshot);
}
