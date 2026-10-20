use nf_contract::{
    identity::UniverseId,
    system_presentation::{ClientPresentation, PresentationDomain, PresentationError},
};

const DOMAINS: [PresentationDomain; 3] = [
    PresentationDomain {
        origin_x: 0,
        origin_y: 0,
        width: 3839,
        height: 3840,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: 0,
        origin_y: 0,
        width: 3840,
        height: 3839,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: 0,
        origin_y: 0,
        width: 255,
        height: 256,
        half_extent: 0,
    },
];

// Fixed complete arguments; no test-side pitch formula or constructor-derived oracle.
const EXPECTED_DOMAIN_VALUES: [(i64, i64, u32, u32, u32); 3] = [
    (0, 0, 3839, 3840, 7),
    (0, 0, 3840, 3839, 7),
    (0, 0, 255, 256, 0),
];

fn domain_values(domain: PresentationDomain) -> (i64, i64, u32, u32, u32) {
    (
        domain.origin_x,
        domain.origin_y,
        domain.width,
        domain.height,
        domain.half_extent,
    )
}

#[test]
fn insufficient_grid_pitch_refuses_without_mutating_arguments() {
    assert_eq!(DOMAINS.len(), 3);
    assert_eq!(EXPECTED_DOMAIN_VALUES.len(), 3);
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        let raw_universe = [0x11_u8; 16];
        let universe = UniverseId::from_bytes(raw_universe);
        let raw_before = raw_universe;
        let universe_before = universe;
        let domain_before = domain;

        assert_eq!(raw_universe, [0x11; 16]);
        assert_eq!(universe.as_bytes(), &[0x11; 16]);
        assert_eq!(domain_values(domain), EXPECTED_DOMAIN_VALUES[index]);

        assert_eq!(
            ClientPresentation::new(universe, domain),
            Err(PresentationError::DoesNotFit),
            "fit partition {index}"
        );

        assert_eq!(raw_universe, raw_before);
        assert_eq!(raw_universe, [0x11; 16]);
        assert_eq!(universe, universe_before);
        assert_eq!(universe.as_bytes(), &[0x11; 16]);
        assert_eq!(domain, domain_before);
        assert_eq!(domain_values(domain), EXPECTED_DOMAIN_VALUES[index]);
    }
}
