use nf_contract::canonical::binding::RequestBinding;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};

fn bytes<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(hex.len(), N * 2);
    core::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}
fn original() -> RequestBinding {
    RequestBinding {
        request_id: RequestId::from_bytes([0x11; 16]),
        account_id: AccountId::from_bytes([0x22; 16]),
        device_id: DeviceId::from_bytes([0x33; 16]),
        universe_id: UniverseId::from_bytes([0x44; 16]),
        history_id: HistoryId::from_bytes([0x55; 16]),
        operation_kind: 1,
        payload_digest: bytes("cad18eda01654e738e671ae3600a09ee9ba809e5237097668bfea13fc1763276"),
    }
}
#[test]
fn request_binding_matches_independent_sha256_golden() {
    assert_eq!(
        original().digest(),
        bytes("f4b3c40b44b559ce916454c230cc074d1c712823a88dcbf2f83202276a4fc36e")
    );
}
#[test]
fn reused_id_cannot_return_original_for_changed_binding_or_intent() {
    let binding = original();
    assert!(binding.verify_retry(&binding, [7; 32], [7; 32]).is_ok());
    for candidate in [
        RequestBinding {
            account_id: AccountId::from_bytes([2; 16]),
            ..binding
        },
        RequestBinding {
            history_id: HistoryId::from_bytes([5; 16]),
            ..binding
        },
        RequestBinding {
            operation_kind: 2,
            ..binding
        },
        RequestBinding {
            payload_digest: [0; 32],
            ..binding
        },
    ] {
        assert!(binding.verify_retry(&candidate, [7; 32], [7; 32]).is_err());
    }
    assert!(binding.verify_retry(&binding, [7; 32], [8; 32]).is_err());
}
