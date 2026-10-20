use nf_contract::{
    identity::UniverseId,
    system_presentation::{ClientPresentation, PresentationDomain, PresentationError},
};

const DOMAINS: [PresentationDomain; 9] = [
    PresentationDomain {
        origin_x: -1_099_511_627_777,
        origin_y: -2048,
        width: 8192,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: 1_099_511_627_777,
        origin_y: -2048,
        width: 8192,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -1_099_511_627_777,
        width: 8192,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: 1_099_511_627_777,
        width: 8192,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 0,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 16_777_217,
        height: 16384,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 8192,
        height: 0,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 8192,
        height: 16_777_217,
        half_extent: 7,
    },
    PresentationDomain {
        origin_x: -1024,
        origin_y: -2048,
        width: 8192,
        height: 16384,
        half_extent: 32_768,
    },
];

// Fixed complete argument values, independent of constructor output or validation logic.
const EXPECTED_DOMAIN_VALUES: [(i64, i64, u32, u32, u32); 9] = [
    (-1_099_511_627_777, -2048, 8192, 16384, 7),
    (1_099_511_627_777, -2048, 8192, 16384, 7),
    (-1024, -1_099_511_627_777, 8192, 16384, 7),
    (-1024, 1_099_511_627_777, 8192, 16384, 7),
    (-1024, -2048, 0, 16384, 7),
    (-1024, -2048, 16_777_217, 16384, 7),
    (-1024, -2048, 8192, 0, 7),
    (-1024, -2048, 8192, 16_777_217, 7),
    (-1024, -2048, 8192, 16384, 32_768),
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
fn numeric_domain_bounds_refuse_before_fit_without_mutating_arguments() {
    assert_eq!(DOMAINS.len(), 9);
    assert_eq!(EXPECTED_DOMAIN_VALUES.len(), 9);
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
            Err(PresentationError::Bounds),
            "numeric partition {index}"
        );

        assert_eq!(raw_universe, raw_before);
        assert_eq!(raw_universe, [0x11; 16]);
        assert_eq!(universe, universe_before);
        assert_eq!(universe.as_bytes(), &[0x11; 16]);
        assert_eq!(domain, domain_before);
        assert_eq!(domain_values(domain), EXPECTED_DOMAIN_VALUES[index]);
    }
}
