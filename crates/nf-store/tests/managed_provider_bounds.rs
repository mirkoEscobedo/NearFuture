use nf_contract::identity::ProviderId;
use nf_store::managed::ManagedError;
use nf_store::migration::{Composition, CompositionError, ProviderVersion};
mod managed_admission_support;
mod support;

fn provider(index: u8) -> ProviderVersion {
    ProviderVersion {
        provider: ProviderId::from_bytes([index; 16]),
        implementation: [1; 32],
        version: 1,
        state_schema: 1,
    }
}

#[test]
fn supported_catalog_allows_the_32_entry_boundary_without_requiring_exact_subset_cardinality() {
    let composition = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![provider(0)]).unwrap();
    let supported: [ProviderVersion; 32] =
        std::array::from_fn(|index| provider(u8::try_from(index).unwrap()));
    assert_eq!(composition.require_supported(&supported), Ok(()));
}

#[test]
fn oversized_preallocated_support_slices_are_refused_at_the_public_support_seam() {
    let composition = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![provider(0)]).unwrap();
    let digest = composition.digest();
    for length in [33, 65_536] {
        let supported = vec![provider(0); length];
        assert_eq!(
            composition.require_supported(&supported),
            Err(CompositionError::Limit),
            "preallocated support length {length}"
        );
        assert_eq!(composition.digest(), digest);
    }
}

#[test]
fn managed_materialization_refuses_incompatible_or_oversized_support_without_persistent_effects() {
    let fixture = managed_admission_support::Fixture::new(1);
    let mut source = fixture.sealed();
    let original = std::fs::read(fixture.scratch.db()).unwrap();
    let catalog = std::fs::read(fixture.catalog()).unwrap();
    let target = fixture.scratch.0.join("target.sqlite");
    let retained = fixture.scratch.0.join("retained.sqlite");
    for length in [0, 2, 32, 33, 65_536] {
        let supported = vec![fixture.supported[0]; length];
        let failure = match source.materialize_active_upgrade(
            &fixture.from,
            &fixture.to,
            &supported,
            target.clone(),
            retained.clone(),
        ) {
            Err(failure) => failure,
            Ok(_) => panic!("incompatible support length {length} admitted"),
        };
        let (error, returned, recovery) = failure.into_parts();
        assert_eq!(
            error,
            if length > 32 {
                ManagedError::Limit
            } else {
                ManagedError::DerivationMismatch
            }
        );
        assert!(recovery.is_none());
        source = returned.expect("known refusal must return the same source authority");
        assert_eq!(std::fs::read(fixture.scratch.db()).unwrap(), original);
        assert_eq!(std::fs::read(fixture.catalog()).unwrap(), catalog);
        assert!(!target.exists());
        assert!(!retained.exists());
    }
}
