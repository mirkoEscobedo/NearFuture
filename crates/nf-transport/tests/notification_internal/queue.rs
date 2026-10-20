use nf_transport::{PeerError, notification::NotifyLimits, notification_effects::NotifyQuota};
#[test]
fn data_only_queue_reserves_exact_body_and_prefix_and_releases_only_its_completion() {
    let mut quota = NotifyQuota::new(NotifyLimits::default()).unwrap();
    let ticket = quota.reserve(493).unwrap();
    assert_eq!(quota.stats().items, 1);
    assert_eq!(quota.stats().body_bytes, 493);
    assert_eq!(quota.stats().framing_bytes, 4);
    quota.complete(ticket).unwrap();
    assert_eq!(quota.stats().items, 0);
    assert_eq!(quota.stats().body_bytes, 0);
    assert_eq!(quota.stats().framing_bytes, 0);
}
#[test]
fn data_only_queue_refuses_hard_frame_item_and_selected_byte_overflow() {
    let limits = NotifyLimits {
        queue_items: 1,
        queue_bytes: 1024,
        ..NotifyLimits::default()
    };
    let mut quota = NotifyQuota::new(limits).unwrap();
    assert!(matches!(quota.reserve(1025), Err(PeerError::Limit)));
    let ticket = quota.reserve(545).unwrap();
    assert!(matches!(quota.reserve(482), Err(PeerError::Backpressure)));
    quota.complete(ticket).unwrap();
    assert!(quota.reserve(482).is_ok());
}
#[test]
fn old_owner_ticket_cannot_release_another_queue_instance() {
    let mut first = NotifyQuota::new(NotifyLimits::default()).unwrap();
    let mut second = NotifyQuota::new(NotifyLimits::default()).unwrap();
    let old = first.reserve(425).unwrap();
    let new = second.reserve(516).unwrap();
    assert!(matches!(second.complete(old), Err(PeerError::Replay)));
    assert_eq!(second.stats().items, 1);
    assert_eq!(second.stats().body_bytes, 516);
    second.complete(new).unwrap();
    first.invalidate();
    assert!(first.reserve(545).is_err());
}

#[test]
fn selected_body_budget_refuses_bytes_even_when_item_capacity_remains() {
    let limits = NotifyLimits {
        queue_bytes: 1024,
        ..NotifyLimits::default()
    };
    let mut quota = NotifyQuota::new(limits).unwrap();
    let held = quota.reserve(545).unwrap();
    assert!(matches!(quota.reserve(482), Err(PeerError::Backpressure)));
    assert_eq!(quota.stats().items, 1);
    quota.complete(held).unwrap();
    assert!(quota.reserve(482).is_ok());
}
