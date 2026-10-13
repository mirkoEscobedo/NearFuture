//! Composition identity commits semantic config independently of provider identity.
use nf_contract::identity::ProviderId;
use nf_store::migration::{Composition, ProviderVersion};

#[test]
fn composition_has_a_fixed_independent_sha256_vector_and_commits_config() {
    let provider = ProviderVersion {
        provider: ProviderId::from_bytes([7; 16]),
        implementation: [4; 32],
        version: 1,
        state_schema: 1,
    };
    let composition = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![provider]).unwrap();
    // Independently assembled fixed-width record, SHA-256 7e903a4f...44ada41.
    let expected = [
        0x7e, 0x90, 0x3a, 0x4f, 0xe6, 0xde, 0x35, 0xd2, 0xa3, 0x24, 0x6b, 0x58, 0x31, 0xac, 0xac,
        0xb5, 0xca, 0xae, 0x15, 0xbb, 0xda, 0xf3, 0x52, 0x8e, 0x9f, 0x83, 0xbc, 0xca, 0xd4, 0x4a,
        0xda, 0x41,
    ];
    assert_eq!(composition.digest(), expected);
    let config_changed = Composition::new(1, [1; 32], [9; 32], [3; 32], vec![provider]).unwrap();
    assert_ne!(composition.digest(), config_changed.digest());
}

#[test]
fn two_provider_identity_is_canonical_and_binds_each_descriptor_field() {
    let low = ProviderVersion {
        provider: ProviderId::from_bytes([1; 16]),
        implementation: [4; 32],
        version: 0x0102_0304,
        state_schema: 0x1122_3344,
    };
    let high = ProviderVersion {
        provider: ProviderId::from_bytes([128; 16]),
        implementation: [5; 32],
        version: 0x0506_0708,
        state_schema: 0x5566_7788,
    };
    // Independent 243-byte fixed-width literal, including nontrivial LE u32 fields.
    let expected = [
        0x58, 0xe2, 0x93, 0x53, 0xbe, 0xa5, 0xc6, 0xdc, 0x41, 0xe4, 0x29, 0x73, 0xf0, 0xdf, 0xdb,
        0xac, 0x73, 0x76, 0xb1, 0x06, 0x52, 0x5c, 0xdf, 0x5d, 0x21, 0xba, 0xb0, 0x4b, 0x2c, 0xbb,
        0x1a, 0xbd,
    ];
    let forward = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![low, high]).unwrap();
    let reverse = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![high, low]).unwrap();
    assert_eq!(forward.digest(), expected);
    assert_eq!(reverse.digest(), expected);

    for changed in [
        ProviderVersion {
            provider: ProviderId::from_bytes([129; 16]),
            ..high
        },
        ProviderVersion {
            implementation: [6; 32],
            ..high
        },
        ProviderVersion {
            version: 0x0506_0709,
            ..high
        },
        ProviderVersion {
            state_schema: 0x5566_7789,
            ..high
        },
    ] {
        let different = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![changed, low]).unwrap();
        assert_ne!(different.digest(), expected);
    }
}
