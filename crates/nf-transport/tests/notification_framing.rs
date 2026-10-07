mod notification_support;
use nf_transport::{
    PeerError,
    notification::{NotifyLimits, admit_frame_length},
};

#[test]
fn frame_length_refuses_hard_and_negotiated_overflow_before_body_work() {
    assert_eq!(
        admit_frame_length([0, 0, 4, 1], NotifyLimits::default()),
        Err(PeerError::Limit)
    );
    let lower = NotifyLimits {
        frame: 768,
        ..NotifyLimits::default()
    };
    assert_eq!(
        admit_frame_length([0, 0, 3, 1], lower),
        Err(PeerError::Limit)
    );
    assert_eq!(admit_frame_length(545u32.to_be_bytes(), lower), Ok(545));
    assert_eq!(
        admit_frame_length(127u32.to_be_bytes(), lower),
        Err(PeerError::Malformed)
    );
}
#[test]
fn independently_declared_missing_bodies_are_refused_by_length_admission() {
    let mut count = 0;
    for row in notification_support::vectors()
        .into_iter()
        .filter(|row| row.category == "frame")
    {
        assert_eq!(row.layer, "limit");
        assert_eq!(row.expectation, "REJECT_LIMIT");
        let cap = if row.name == "selected-over-with-missing-body" {
            768
        } else {
            1024
        };
        assert_eq!(
            admit_frame_length(
                row.bytes.try_into().unwrap(),
                NotifyLimits {
                    frame: cap,
                    ..NotifyLimits::default()
                }
            ),
            Err(PeerError::Limit),
            "{}",
            row.name
        );
        count += 1;
    }
    assert_eq!(count, 2);
}
