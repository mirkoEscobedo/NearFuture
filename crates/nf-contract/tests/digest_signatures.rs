use nf_contract::signatures::{public_key_from_seed, sign_digest, verify_digest};

#[test]
fn a_signature_is_bound_to_the_exact_canonical_digest() {
    // Public RFC8032 test seed, never a credential.
    let seed = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let digest = [7; 32];
    let signed = sign_digest(&seed, &digest);
    assert!(verify_digest(&signed.public_key, &digest, &signed.signature).is_ok());
    assert!(verify_digest(&signed.public_key, &[8; 32], &signed.signature).is_err());
    assert!(verify_digest(&[0; 32], &digest, &signed.signature).is_err());
    assert!(verify_digest(&signed.public_key, &digest, &[0; 64]).is_err());
}

mod support;
#[test]
fn ed25519_matches_independent_shared_digest_signatures() {
    for vector in support::corpus()["signatures"].as_array().unwrap() {
        let seed: [u8; 32] = support::bytes(vector["test_seed_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let digest: [u8; 32] = support::bytes(vector["canonical_digest_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let expected_key: [u8; 32] = support::bytes(vector["public_key_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let expected_signature: [u8; 64] =
            support::bytes(vector["signature_hex"].as_str().unwrap())
                .try_into()
                .unwrap();
        assert_eq!(
            public_key_from_seed(&seed),
            expected_key,
            "{}",
            vector["name"]
        );
        let actual = sign_digest(&seed, &digest);
        assert_eq!(actual.public_key, expected_key, "{}", vector["name"]);
        assert_eq!(actual.signature, expected_signature, "{}", vector["name"]);
        assert!(verify_digest(&expected_key, &digest, &expected_signature).is_ok());
    }
}

#[test]
fn mixed_torsion_points_are_outside_the_shared_signature_profile() {
    let key: [u8; 32] =
        support::bytes("98519eadf35b995233b51b5cd23e9cc5a28b639b5a4af0ec903cb960d81b7819")
            .try_into()
            .unwrap();
    for (counter, signature) in [
        (
            1,
            "586666666666666666666666666666666666666666666666666666666666666641f7d1fa3173fd39e1e32795778945fb4d7d340456be9875ee976b037cd91903",
        ),
        (
            4,
            "98519eadf35b995233b51b5cd23e9cc5a28b639b5a4af0ec903cb960d81b7819d857ddf5e552a8465f8e4c1982a2b06bdfa3d3935540f40148dd3429a57ec10b",
        ),
    ] {
        let mut digest = [0; 32];
        digest[0] = counter;
        let signature: [u8; 64] = support::bytes(signature).try_into().unwrap();
        assert!(verify_digest(&key, &digest, &signature).is_err());
    }
}

#[test]
fn every_shared_adversarial_signature_is_rejected() {
    for vector in support::corpus()["negative_signatures"].as_array().unwrap() {
        let key: Result<[u8; 32], _> =
            support::bytes(vector["public_key_hex"].as_str().unwrap()).try_into();
        let digest: Result<[u8; 32], _> =
            support::bytes(vector["canonical_digest_hex"].as_str().unwrap()).try_into();
        let signature: Result<[u8; 64], _> =
            support::bytes(vector["signature_hex"].as_str().unwrap()).try_into();
        // Width is enforced by the typed public seam before library verification.
        if let (Ok(key), Ok(digest), Ok(signature)) = (key, digest, signature) {
            assert!(
                verify_digest(&key, &digest, &signature).is_err(),
                "{}",
                vector["name"]
            );
        }
    }
}
