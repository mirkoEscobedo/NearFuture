use nf_transport::notification::NotifyLimits;

#[test]
fn notification_negotiation_is_exact_componentwise_minimum() {
    let client = NotifyLimits {
        frame: 768,
        queue_bytes: 12288,
        queue_items: 6,
        rate: 3,
        burst: 8,
        pending: 1,
    };
    let server = NotifyLimits {
        frame: 1024,
        queue_bytes: 8192,
        queue_items: 4,
        rate: 8,
        burst: 2,
        pending: 1,
    };
    let expected = NotifyLimits {
        frame: 768,
        queue_bytes: 8192,
        queue_items: 4,
        rate: 3,
        burst: 2,
        pending: 1,
    };
    assert_eq!(client.negotiate(server).unwrap(), expected);
    assert_eq!(server.negotiate(client).unwrap(), expected);
}
