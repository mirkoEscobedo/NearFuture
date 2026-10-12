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
        0x7e, 0x90, 0x3a, 0x4f, 0xe6, 0xde, 0x35, 0xd2, 0xa3, 0x24, 0x6b, 0x58, 0x31, 0xac,
        0xac, 0xb5, 0xca, 0xae, 0x15, 0xbb, 0xda, 0xf3, 0x52, 0x8e, 0x9f, 0x83, 0xbc, 0xca,
        0xd4, 0x4a, 0xda, 0x41,
    ];
    assert_eq!(composition.digest(), expected);
    let config_changed = Composition::new(1, [1; 32], [9; 32], [3; 32], vec![provider]).unwrap();
    assert_ne!(composition.digest(), config_changed.digest());
}
