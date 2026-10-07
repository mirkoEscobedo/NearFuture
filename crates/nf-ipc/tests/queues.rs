use nf_ipc::{BoundedQueues, IpcError, Lane, QueueLimits};
#[test]
fn independent_lanes_reserve_control_and_retry_without_dropping() {
    let limits = QueueLimits {
        control_bytes: 8,
        bulk_bytes: 12,
        control_items: 2,
        bulk_items: 2,
    };
    let mut queues = BoundedQueues::new(limits).unwrap();
    queues.try_push(Lane::Bulk, vec![2; 12]).unwrap();
    assert_eq!(
        queues.try_push(Lane::Bulk, vec![3]),
        Err(IpcError::Backpressure)
    );
    queues.try_push(Lane::Control, vec![1; 8]).unwrap();
    assert_eq!(queues.bytes(), 20);
    assert_eq!(queues.pop(), Some((Lane::Control, vec![1; 8])));
    assert_eq!(queues.pop(), Some((Lane::Bulk, vec![2; 12])));
    assert_eq!(queues.bytes(), 0);
    queues.try_push(Lane::Bulk, vec![3]).unwrap();
    queues.clear();
    assert_eq!(queues.pop(), None);
    assert_eq!(queues.bytes(), 0);
}
