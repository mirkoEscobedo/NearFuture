mod miniature_support;
use nf_kernel::miniature::*;
#[test]
fn immutable_intent_has_independent_exact_golden_without_fresh_policy_frontier() {
    let intent = miniature_support::intent(&miniature_support::world());
    let expected = miniature_support::hex(include_str!("fixtures/miniature/intent-colony.hex"));
    assert_eq!(
        encode_miniature_intent(&intent).expect("supported miniature intent"),
        expected
    );
    assert_eq!(expected.len(), 230);
    assert!(nf_kernel::decode_intent(&expected).is_err());
}
#[test]
fn intent_decoder_and_retry_binding_preserve_device_action_and_expected_revisions() {
    let original = miniature_support::intent(&miniature_support::world());
    let bytes = miniature_support::hex(include_str!("fixtures/miniature/intent-colony.hex"));
    assert_eq!(
        decode_miniature_intent(&bytes).expect("supported exact intent decoder"),
        original
    );
    let binding = miniature_request_binding(&original).expect("supported immutable binding");
    assert_eq!(binding.operation_kind, 4);
    assert_eq!(
        binding.payload_digest,
        miniature_support::hex("fde7f049935f5aaafe208fe678226a4b7863f13291b6d9791fd534091f2b0b22")
            .as_slice()
    );
    for mode in 0..3 {
        let mut changed = original.clone();
        match mode {
            0 => changed.device = nf_contract::identity::DeviceId::from_bytes([34; 16]),
            1 => {
                changed.action = nf_world::WorldAction::Travel {
                    fleet: nf_contract::identity::EntityId::from_bytes([8; 16]),
                    destination: nf_contract::identity::EntityId::from_bytes([9; 16]),
                }
            }
            _ => {
                *changed.expected.values_mut().next().unwrap() =
                    nf_contract::identity::AggregateRevision(1)
            }
        }
        let other = miniature_request_binding(&changed).unwrap();
        assert!(
            binding
                .verify_retry(&other, binding.payload_digest, other.payload_digest)
                .is_err()
        );
    }
    let mut invalid = bytes.clone();
    invalid[197] = 5;
    assert!(decode_miniature_intent(&invalid).is_err());
    let mut reversed = bytes.clone();
    let first = reversed[149..173].to_vec();
    let second = reversed[173..197].to_vec();
    reversed[149..173].copy_from_slice(&second);
    reversed[173..197].copy_from_slice(&first);
    assert!(decode_miniature_intent(&reversed).is_err());
}
