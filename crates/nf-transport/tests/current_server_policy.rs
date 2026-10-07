use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::generate_identity, model::*, rotation::revocation_digest, signing::device_digest,
};
use nf_transport::{
    PeerError,
    auth::{ClientHandshake, HandshakeContext, ServerPin},
    records::{Lane, PeerContext, PeerLimits},
};
fn setup() -> (
    HandshakeContext,
    MembershipState,
    nf_identity::keys::SecretSeed,
    nf_identity::keys::SecretSeed,
) {
    let cp = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let sp = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let (public, account, device) = generate_identity(sp.to_bytes()).unwrap();
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let state = MembershipState::bootstrap(scope, &public).unwrap();
    let ctx = HandshakeContext {
        lane: Lane::Control,
        client_peer: cp,
        server_peer: sp,
        client_account: public.account,
        client_device: public.device,
        server_account: public.account,
        server_device: public.device,
        context: PeerContext {
            session: [3; 16],
            scope,
            ruleset: [4; 32],
            content: [5; 32],
        },
        client_nonce: [6; 32],
        server_nonce: [7; 32],
        required: 1,
        optional: 2,
        server_available: 3,
        selected_caps: 3,
        offered: PeerLimits::default(),
        server_limits: PeerLimits::default(),
        selected: PeerLimits::default(),
    };
    (ctx, state, account, device)
}
fn proof(
    c: &HandshakeContext,
    s: &MembershipState,
    key: &nf_identity::keys::SecretSeed,
    stage: u8,
) -> DeviceProof {
    let mut p = DeviceProof {
        scope: s.scope,
        account: c.server_account,
        device: c.server_device,
        frontier: s.revision,
        peer: c.server_peer.to_bytes(),
        challenge: c.challenge(stage, s.revision).unwrap(),
        signature: [0; 64],
    };
    p.signature = key.sign(&device_digest(&p).unwrap());
    p
}
fn client(c: &HandshakeContext) -> ClientHandshake {
    ClientHandshake::new(
        c.clone(),
        ServerPin {
            peer: c.server_peer,
            account: c.server_account,
            device: c.server_device,
            minimum_membership: 0,
        },
    )
    .unwrap()
}
#[test]
fn current_policy_is_an_explicit_each_stage_input_and_first_failed_attempt_consumes() {
    let (c, s, ak, dk) = setup();
    let mut verifier = client(&c);
    verifier
        .accept_server_hello(&proof(&c, &s, &dk, 1), &s)
        .unwrap();
    assert!(!verifier.active());
    verifier
        .accept_finished(&proof(&c, &s, &dk, 3), &s)
        .unwrap();
    assert!(verifier.active());
    assert_eq!(
        verifier.accept_finished(&proof(&c, &s, &dk, 3), &s),
        Err(PeerError::Replay)
    );
    let mut verifier = client(&c);
    let mut forged = proof(&c, &s, &dk, 1);
    forged.signature[0] ^= 1;
    assert_eq!(
        verifier.accept_server_hello(&forged, &s),
        Err(PeerError::Unauthorized)
    );
    assert_eq!(
        verifier.accept_server_hello(&proof(&c, &s, &dk, 1), &s),
        Err(PeerError::Replay)
    );
    let mut verifier = client(&c);
    verifier
        .accept_server_hello(&proof(&c, &s, &dk, 1), &s)
        .unwrap();
    let revoke = DeviceRevocation {
        scope: s.scope,
        issuer: s.owner,
        device: c.server_device,
        frontier: s.revision,
    };
    let revoked = s
        .revoke_device(&revoke, &ak.sign(&revocation_digest(&revoke)))
        .unwrap();
    assert_eq!(
        verifier.accept_finished(&proof(&c, &s, &dk, 3), &revoked),
        Err(PeerError::Unauthorized)
    );
    assert!(!verifier.active());
}
