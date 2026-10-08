use nf_contract::identity::{AccountId, DeviceId, HistoryId, OperationId, RequestId, UniverseId};
use nf_kernel::supplies::*;

fn value() -> Reserve {
    Reserve {
        request: RequestId::from_bytes([1; 16]),
        reservation: OperationId::from_bytes([2; 16]),
        actor: AccountId::from_bytes([3; 16]),
        device: DeviceId::from_bytes([4; 16]),
        owner: AccountId::from_bytes([3; 16]),
        universe: UniverseId::from_bytes([5; 16]),
        history: HistoryId::from_bytes([6; 16]),
        policy: [7; 32],
        content: SUPPLIES_CONTENT,
        origin: Origin {
            trust: TrustClass::Canonical,
            lineage: [8; 32],
        },
        amount: 25,
    }
}
#[test]
fn canonical_reserve_is_exactly242_bytes_and_roundtrips_each_closed_trust_class() {
    for trust in [
        TrustClass::Sandbox,
        TrustClass::CooperativeAudited,
        TrustClass::Canonical,
    ] {
        let mut original = value();
        original.origin.trust = trust;
        let bytes = reserve_bytes(&original);
        assert_eq!(bytes.len(), 242);
        assert_eq!(decode_reserve(&bytes), Ok(original));
    }
}
#[test]
fn malformed_reserve_prefix_truncation_trailing_bytes_and_unknown_trust_are_refused() {
    let bytes = reserve_bytes(&value());
    for length in 0..242 {
        assert_eq!(
            decode_reserve(&bytes[..length]),
            Err(SuppliesRejection::Malformed)
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(decode_reserve(&trailing), Err(SuppliesRejection::Malformed));
    let mut prefix = bytes.clone();
    prefix[0] ^= 1;
    assert_eq!(decode_reserve(&prefix), Err(SuppliesRejection::Malformed));
    // Literal width:22-byte domain +112 bytes of IDs +32 policy +32 content =198.
    for unknown in [0_u32, 4, u32::MAX] {
        let mut unknown_trust = bytes.clone();
        unknown_trust[198..202].copy_from_slice(&unknown.to_le_bytes());
        assert_eq!(
            decode_reserve(&unknown_trust),
            Err(SuppliesRejection::Malformed)
        );
    }
}
#[test]
fn request_alias_changes_full_binding_but_preserves_only_economic_reservation_identity() {
    let original = value();
    let mut alias = original;
    alias.request = RequestId::from_bytes([9; 16]);
    assert_ne!(reserve_bytes(&alias), reserve_bytes(&original));
    assert_ne!(
        reserve_binding(&alias).digest(),
        reserve_binding(&original).digest()
    );
    assert_eq!(
        economic_reserve_digest(&alias),
        economic_reserve_digest(&original)
    );
    assert_eq!(reserve_binding(&original).operation_kind, 0x5355_0003_u32);
    let issuance = Issuance {
        request: original.request,
        issuance: original.reservation,
        actor: original.actor,
        device: original.device,
        beneficiary: original.owner,
        universe: original.universe,
        history: original.history,
        policy: original.policy,
        content: original.content,
        origin: original.origin,
        reason: IssuanceReason::AuthorityGrant,
        amount: original.amount,
    };
    let burn = Burn {
        request: original.request,
        burn: original.reservation,
        actor: original.actor,
        device: original.device,
        owner: original.owner,
        universe: original.universe,
        history: original.history,
        policy: original.policy,
        content: original.content,
        origin: original.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: original.amount,
    };
    assert_ne!(
        economic_reserve_digest(&original),
        economic_issuance_digest(&issuance)
    );
    assert_ne!(
        economic_reserve_digest(&original),
        economic_burn_digest(&burn)
    );
}
#[test]
fn economic_reservation_identity_binds_every_field_except_request_id() {
    let original = value();
    let mut variants = Vec::new();
    let mut changed = original;
    changed.reservation = OperationId::from_bytes([20; 16]);
    variants.push(("reservation", changed));
    let mut changed = original;
    changed.actor = AccountId::from_bytes([21; 16]);
    variants.push(("actor", changed));
    let mut changed = original;
    changed.device = DeviceId::from_bytes([22; 16]);
    variants.push(("device", changed));
    let mut changed = original;
    changed.owner = AccountId::from_bytes([23; 16]);
    variants.push(("owner", changed));
    let mut changed = original;
    changed.universe = UniverseId::from_bytes([24; 16]);
    variants.push(("universe", changed));
    let mut changed = original;
    changed.history = HistoryId::from_bytes([25; 16]);
    variants.push(("history", changed));
    let mut changed = original;
    changed.policy[0] ^= 1;
    variants.push(("policy", changed));
    let mut changed = original;
    changed.content[0] ^= 1;
    variants.push(("content", changed));
    let mut changed = original;
    changed.origin.trust = TrustClass::Sandbox;
    variants.push(("trust", changed));
    let mut changed = original;
    changed.origin.lineage[0] ^= 1;
    variants.push(("lineage", changed));
    let mut changed = original;
    changed.amount += 1;
    variants.push(("amount", changed));
    for (field, changed) in variants {
        assert_ne!(
            economic_reserve_digest(&changed),
            economic_reserve_digest(&original),
            "{field}"
        );
        assert_ne!(
            reserve_binding(&changed).digest(),
            reserve_binding(&original).digest(),
            "{field}"
        );
    }
}
