use nf_transport::{
    PeerError,
    budget::ResponseBudget,
    records::{Lane, PeerLimits},
};
#[test]
fn response_queue_reserves_before_growth_releases_exact_owned_slot_and_never_drops() {
    let l = PeerLimits {
        control_frame: 1024,
        control_queue_bytes: 1024,
        control_items: 1,
        ..PeerLimits::default()
    };
    let mut q = ResponseBudget::new(Lane::Control, l).unwrap();
    let first = q.reserve(500).unwrap();
    assert_eq!(q.reserve(1), Err(PeerError::Backpressure));
    assert_eq!(q.pending(), (1, 500));
    q.release(first).unwrap();
    assert_eq!(q.pending(), (0, 0));
    assert_eq!(q.release(first), Err(PeerError::Replay));
    let mut other = ResponseBudget::new(Lane::Control, l).unwrap();
    let other_slot = other.reserve(123).unwrap();
    assert_eq!(other.release(first), Err(PeerError::Replay));
    assert_eq!(other.pending(), (1, 123));
    other.release(other_slot).unwrap();
    let _second = q.reserve(1024).unwrap();
    assert_eq!(q.reserve(1), Err(PeerError::Backpressure));
}
