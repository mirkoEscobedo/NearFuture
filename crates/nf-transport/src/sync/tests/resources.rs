use super::{super::*, support::*};
use crate::PeerError;
#[test]
fn hard_and_selected_caps_are_checked_before_missing_payload() {
    let p = row("sync-chunk-lane2").policy();
    for n in [8393, 9216, u32::MAX] {
        assert!(matches!(
            decode_frame(&n.to_be_bytes(), p),
            Err(PeerError::Limit)
        ));
    }
    let lower = SyncWirePolicy {
        limits: SyncLimits {
            transfer_frame: 1024,
            chunk_bytes: 256,
            ..p.limits
        },
        ..p
    };
    assert!(matches!(
        decode_frame(&1025u32.to_be_bytes(), lower),
        Err(PeerError::Limit)
    ));
    let mut chunk = row("chunk-first-selected256").bytes();
    chunk.truncate(200);
    chunk[198..200].copy_from_slice(&257u16.to_le_bytes());
    assert!(matches!(decode_body(&chunk, lower), Err(PeerError::Limit)));
    let p = row("hello-lane1").policy();
    assert!(matches!(
        decode_frame(&1025u32.to_be_bytes(), p),
        Err(PeerError::Limit)
    ));
}
#[test]
fn limits_only_lower_hard_bounds_and_document_horizon_is_checked() {
    let valid = SyncLimits::default();
    assert_eq!(valid.document_maximum().unwrap(), 2105344);
    assert!(valid.document(2105344, 257).is_ok());
    assert!(valid.document(2105345, 257).is_err());
    let lower = SyncLimits {
        transfer_frame: 1024,
        chunk_bytes: 256,
        ..valid
    };
    assert_eq!(lower.document_maximum().unwrap(), 65792);
    assert!(lower.document(65792, 257).is_ok());
    assert!(lower.document(65793, 257).is_err());
    assert!(lower.chunk(391, 2, 0, 256).is_ok());
    assert!(lower.chunk(391, 2, 1, 135).is_ok());
    assert!(lower.chunk(391, 2, 1, 136).is_err());
    assert!(lower.chunk(391, 2, 2, 135).is_err());
    for invalid in [
        SyncLimits {
            chunk_bytes: 8193,
            ..valid
        },
        SyncLimits {
            transfer_frame: 9217,
            ..valid
        },
        SyncLimits {
            control_frame: 1025,
            ..valid
        },
        SyncLimits {
            control_items: 5,
            ..valid
        },
        SyncLimits {
            transfer_items: 3,
            ..valid
        },
        SyncLimits {
            pending_challenges: 2,
            ..valid
        },
        SyncLimits {
            chunk_bytes: 825,
            transfer_frame: 1024,
            ..valid
        },
    ] {
        assert!(invalid.validate().is_err());
        assert!(invalid.negotiate(valid).is_err());
        let p = SyncWirePolicy {
            limits: invalid,
            ..row("hello-lane1").policy()
        };
        assert!(matches!(decode_frame(&[], p), Err(PeerError::Limit)));
    }
    assert_eq!(valid.negotiate(lower).unwrap(), lower);
}
