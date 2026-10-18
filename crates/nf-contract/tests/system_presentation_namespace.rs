use nf_contract::{
    identity::UniverseId,
    system_presentation::{ClientPresentation, PresentationDomain, PresentationError},
};

#[test]
fn zero_namespace_refuses_before_numeric_bounds_without_mutating_arguments() {
    let raw_universe = [0_u8; 16];
    let universe = UniverseId::from_bytes(raw_universe);
    let domain = PresentationDomain {
        origin_x: 0,
        origin_y: 0,
        width: 0,
        height: 0,
        half_extent: 0,
    };
    let raw_before = raw_universe;
    let universe_before = universe;
    let domain_before = domain;

    assert_eq!(raw_universe, [0; 16]);
    assert_eq!(universe.as_bytes(), &[0; 16]);
    assert_eq!(domain.origin_x, 0);
    assert_eq!(domain.origin_y, 0);
    assert_eq!(domain.width, 0);
    assert_eq!(domain.height, 0);
    assert_eq!(domain.half_extent, 0);

    // The zero namespace must refuse before the independently invalid dimensions.
    assert_eq!(
        ClientPresentation::new(universe, domain),
        Err(PresentationError::InvalidNamespace)
    );

    assert_eq!(raw_universe, raw_before);
    assert_eq!(raw_universe, [0; 16]);
    assert_eq!(universe, universe_before);
    assert_eq!(universe.as_bytes(), &[0; 16]);
    assert_eq!(domain, domain_before);
    assert_eq!(domain.origin_x, 0);
    assert_eq!(domain.origin_y, 0);
    assert_eq!(domain.width, 0);
    assert_eq!(domain.height, 0);
    assert_eq!(domain.half_extent, 0);
}
