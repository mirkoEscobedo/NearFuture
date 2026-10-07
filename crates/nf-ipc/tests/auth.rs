use nf_contract::identity::{AccountId, DeviceId};
use nf_ipc::*;
fn binding() -> AuthBinding {
    AuthBinding {
        config: SessionConfig {
            universe: [1; 16],
            history: [2; 16],
            runtime_session: 9,
            ruleset: [3; 32],
            content_policy: [4; 32],
            limits: default_limits(),
        },
        principal: LocalPrincipal {
            account: AccountId::from_bytes([5; 16]),
            device: DeviceId::from_bytes([6; 16]),
        },
        role: EndpointRole::Control,
        port: 12345,
    }
}
#[test]
fn fixed_transcript_matches_independent_node_hmac_and_domains_are_distinct() {
    let b = binding();
    let token = SecretToken::from_bytes(core::array::from_fn(|i| i as u8));
    let expected = [
        "466f94bb04b5956d791c8afd807185ad0f66ca8e1d7e5c6a0b15ae22bfc5a767",
        "004c79ccb227145ba20ad9e154276340a2e51a8e6d4f537c01cee5730015ed0e",
        "c29c2d40fadbee4770c76775de39f6fae8876807c1f9a49a26d235ad785eba5d",
    ];
    for (index, stage) in [
        ProofStage::ServerChallenge,
        ProofStage::ClientProof,
        ProofStage::ServerFinished,
    ]
    .into_iter()
    .enumerate()
    {
        let transcript = auth_transcript(
            &b,
            b.config.limits,
            b.config.limits,
            [8; 32],
            [9; 32],
            stage,
        )
        .unwrap();
        assert_eq!(transcript.len(), 306);
        let proof = auth_proof(&token, &transcript).unwrap();
        let hex = proof.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(hex, expected[index]);
    }
}
#[test]
fn mutual_proof_rejects_impostor_reflection_replay_and_legacy_envelope() {
    let b = binding();
    let auth = Authenticator::new(b.clone(), SecretToken::from_bytes([7; 32])).unwrap();
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    let proof = client.respond(challenge.challenge()).unwrap();
    let (_, accepted) = auth.finish(challenge, proof.proof()).unwrap();
    assert_eq!(proof.finish(&accepted).unwrap().runtime_session(), 9);
    let impostor = Authenticator::new(b.clone(), SecretToken::from_bytes([8; 32])).unwrap();
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let wrong = impostor.begin(client.hello()).unwrap();
    assert!(matches!(
        client.respond(wrong.challenge()),
        Err(IpcError::Unauthorized)
    ));
    assert!(matches!(auth.begin(&[8, 1]), Err(IpcError::Malformed)));
}
#[test]
fn server_port_role_and_new_challenge_reject_old_or_reflected_proof() {
    let b = binding();
    let auth = Authenticator::new(b.clone(), SecretToken::from_bytes([7; 32])).unwrap();
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let first = auth.begin(client.hello()).unwrap();
    let proof = client.respond(first.challenge()).unwrap();
    let old = proof.proof().to_vec();
    auth.finish(first, &old).unwrap();
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let second = auth.begin(client.hello()).unwrap();
    assert!(matches!(
        auth.finish(second, &old),
        Err(IpcError::Unauthorized)
    ));
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let third = auth.begin(client.hello()).unwrap();
    let reflected = third.challenge().to_vec();
    assert!(matches!(
        auth.finish(third, &reflected),
        Err(IpcError::Unauthorized)
    ));
    let mut wrong = b.clone();
    wrong.port += 1;
    let client =
        ClientHello::start(wrong, SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    assert!(matches!(
        client.respond(challenge.challenge()),
        Err(IpcError::Unauthorized)
    ));
    let mut wrong = b.clone();
    wrong.role = EndpointRole::Bulk;
    let client =
        ClientHello::start(wrong, SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    assert!(matches!(
        auth.begin(client.hello()),
        Err(IpcError::Unauthorized)
    ));
    assert_eq!(
        format!("{:?}", SecretToken::from_bytes([7; 32])),
        "SecretToken([REDACTED])"
    );
}
#[test]
fn context_and_negotiated_limits_are_bound_before_client_activation() {
    use nf_wire::local_auth as a;
    use prost::Message;
    let b = binding();
    let auth = Authenticator::new(b.clone(), SecretToken::from_bytes([7; 32])).unwrap();
    for field in 0..7 {
        let mut other = b.clone();
        match field {
            0 => other.config.universe[0] ^= 1,
            1 => other.config.history[0] ^= 1,
            2 => other.config.ruleset[0] ^= 1,
            3 => other.config.content_policy[0] ^= 1,
            4 => other.principal.account = AccountId::from_bytes([55; 16]),
            5 => other.principal.device = DeviceId::from_bytes([66; 16]),
            _ => other.config.runtime_session += 1,
        }
        let client = ClientHello::start(
            other.clone(),
            SecretToken::from_bytes([7; 32]),
            other.config.limits,
        )
        .unwrap();
        match auth.begin(client.hello()) {
            Err(IpcError::Unauthorized | IpcError::SessionMismatch) => {}
            Ok(challenge) => assert!(matches!(
                client.respond(challenge.challenge()),
                Err(IpcError::Unauthorized | IpcError::PolicyMismatch)
            )),
            _ => panic!("context mutation accepted unexpectedly"),
        }
    }
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let mut altered = nf_wire::decode_local_auth(client.hello()).unwrap();
    let Some(a::local_auth_envelope::Body::Hello(h)) = &mut altered.body else {
        panic!("hello")
    };
    h.offered_limits.as_mut().unwrap().control_frame_bytes /= 2;
    let challenge = auth.begin(&altered.encode_to_vec()).unwrap();
    assert!(matches!(
        client.respond(challenge.challenge()),
        Err(IpcError::PolicyMismatch)
    ));
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    let mut tampered = nf_wire::decode_local_auth(challenge.challenge()).unwrap();
    let Some(a::local_auth_envelope::Body::Challenge(c)) = &mut tampered.body else {
        panic!("challenge")
    };
    c.selected_limits.as_mut().unwrap().control_frame_bytes /= 2;
    assert!(matches!(
        client.respond(&tampered.encode_to_vec()),
        Err(IpcError::PolicyMismatch)
    ));
    let client =
        ClientHello::start(b.clone(), SecretToken::from_bytes([7; 32]), b.config.limits).unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    let proof = client.respond(challenge.challenge()).unwrap();
    let (_, finished) = auth.finish(challenge, proof.proof()).unwrap();
    let mut tampered = nf_wire::decode_local_auth(&finished).unwrap();
    let Some(a::local_auth_envelope::Body::Accepted(a)) = &mut tampered.body else {
        panic!("finished")
    };
    a.server_finished_proof[0] ^= 1;
    assert!(matches!(
        proof.finish(&tampered.encode_to_vec()),
        Err(IpcError::Unauthorized)
    ));
}
