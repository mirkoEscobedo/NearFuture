use nf_transport::{PeerError, notification::NotifyLimits, notification_effects::NotifyFlow};
#[test]
fn all_frame_rate_budget_cannot_be_reset_by_releasing_queue_tickets() {
    let mut flow = NotifyFlow::new(NotifyLimits::default()).unwrap();
    for bytes in [214, 493, 425, 425, 251, 248, 473, 516] {
        let permit = flow.reserve_outbound(bytes).unwrap();
        flow.complete_outbound(permit).unwrap();
    }
    assert!(matches!(
        flow.reserve_outbound(545),
        Err(PeerError::Backpressure)
    ));
    assert_eq!(flow.outbound_stats().items, 0);
}
#[test]
fn inbound_rate_refusal_occurs_without_enqueuing_a_ninth_frame() {
    let mut flow = NotifyFlow::new(NotifyLimits::default()).unwrap();
    for _ in 0..8 {
        let permit = flow.admit_inbound(545).unwrap();
        flow.complete_inbound(permit).unwrap();
    }
    assert!(matches!(
        flow.admit_inbound(545),
        Err(PeerError::Backpressure)
    ));
    assert_eq!(flow.inbound_stats().items, 0);
    assert_eq!(flow.inbound_stats().highwater_items, 1);
}
#[test]
fn monotonic_refill_is_capped_at_selected_burst() {
    let limits = NotifyLimits {
        rate: 8,
        burst: 1,
        ..NotifyLimits::default()
    };
    let mut flow = NotifyFlow::new(limits).unwrap();
    let permit = flow.reserve_outbound(214).unwrap();
    flow.complete_outbound(permit).unwrap();
    assert!(flow.reserve_outbound(214).is_err());
    std::thread::sleep(std::time::Duration::from_millis(150));
    let permit = flow.reserve_outbound(214).unwrap();
    flow.complete_outbound(permit).unwrap();
    assert!(flow.reserve_outbound(214).is_err());
}
