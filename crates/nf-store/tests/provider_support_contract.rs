//! Explicit caller-supplied descriptor support never selects an older fallback.
use nf_contract::identity::ProviderId;
use nf_store::migration::{Composition, CompositionError, ProviderVersion};

#[test]
fn required_provider_and_state_versions_need_explicit_exact_support() {
    let known = ProviderVersion {
        provider: ProviderId::from_bytes([7; 16]),
        implementation: [0; 32], // Opaque supplied bytes, not binary authenticity.
        version: 1,
        state_schema: 1,
    };
    let required = ProviderVersion {
        provider: ProviderId::from_bytes([9; 16]),
        implementation: [4; 32],
        version: 2,
        state_schema: 3,
    };
    let composition =
        Composition::new(1, [1; 32], [2; 32], [3; 32], vec![known, required]).unwrap();
    let digest_before = composition.digest();
    let older_version = [
        known,
        ProviderVersion {
            version: 1,
            ..required
        },
    ];
    let older_schema = [
        known,
        ProviderVersion {
            state_schema: 2,
            ..required
        },
    ];
    let other_implementation = [
        known,
        ProviderVersion {
            implementation: [5; 32],
            ..required
        },
    ];
    let exact = [required, known];
    let inputs_before = (older_version, older_schema, other_implementation, exact);

    assert_eq!(
        composition.require_supported(&[]),
        Err(CompositionError::UnsupportedProvider)
    );
    assert_eq!(
        composition.require_supported(&older_version),
        Err(CompositionError::UnsupportedProvider)
    );
    assert_eq!(
        composition.require_supported(&older_schema),
        Err(CompositionError::UnsupportedProvider)
    );
    assert_eq!(
        composition.require_supported(&other_implementation),
        Err(CompositionError::UnsupportedProvider)
    );
    assert_eq!(composition.require_supported(&exact), Ok(()));
    assert_eq!(composition.digest(), digest_before);
    assert_eq!(
        (older_version, older_schema, other_implementation, exact),
        inputs_before
    );
}
