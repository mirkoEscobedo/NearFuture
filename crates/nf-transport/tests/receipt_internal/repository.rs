#[path = "../receipt_effect_support/fixture.rs"]
mod fixture;
#[path = "../receipt_effect_support/fork.rs"]
mod fork;
#[path = "../receipt_effect_support/scratch.rs"]
mod scratch;
use nf_contract::identity::EventSeq;
use nf_transport::{receipt::ReceiptPhase, receipt_effects::*};
#[test]
fn actual_sql_receipt_observes_one_real_commit_and_identical_retry_consumes_no_generation() {
    let mut fixture = fixture::Fixture::new();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (client_session, server_session) = fixture.sessions(id);
    let (original, minimum) = fixture.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(client_session, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(server_session).unwrap();
    let sp = libp2p::PeerId::from_bytes(&fixture.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&fixture.client_public.peer).unwrap();
    let begin = client.begin(&mut fixture.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut fixture.server).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &mut fixture.client)
        .unwrap();
    let response = server.prove(proof, cp, id, &mut fixture.server).unwrap();
    let verified = client.reply(response, sp, id, &mut fixture.client).unwrap();
    let AdmittedReceipt::Status(pending) = fixture.client.accept_verified(0, verified).unwrap()
    else {
        panic!("pending")
    };
    assert_eq!(pending.phase, ReceiptPhase::Pending);
    let ack = fixture
        .server
        .commit_trusted_prepared(fixture.commit.take().unwrap())
        .unwrap();
    assert_eq!(ack.sequence(), EventSeq(1));
    let committed_database = std::fs::read(fixture.scratch.root.join("server.sqlite")).unwrap();
    for _ in 0..2 {
        // Component records do not prove backend completion. Use a new live handshake per
        // component retry; receipt_operation_lane covers actual same-session custody.
        let (client_session, server_session) = fixture.sessions(id);
        let mut client =
            ReceiptOperationClient::new(client_session, id, fixture.original.clone(), minimum)
                .unwrap();
        let mut server = ReceiptOperationServer::new(server_session).unwrap();
        let begin = client.begin(&mut fixture.client).unwrap();
        let nf_transport::receipt::ReceiptBody::Begin { minimum: sent, .. } = &begin.body else {
            panic!("begin");
        };
        assert!(
            sent.admits(fixture.client.book_anchors()[0].minimum),
            "new query must carry persisted protected minima"
        );
        let challenge = server.begin(begin, cp, id, &mut fixture.server).unwrap();
        let proof = client
            .challenge(challenge, sp, id, &mut fixture.client)
            .unwrap();
        let response = server.prove(proof, cp, id, &mut fixture.server).unwrap();
        let verified = client.reply(response, sp, id, &mut fixture.client).unwrap();
        let AdmittedReceipt::Status(status) = fixture.client.accept_verified(0, verified).unwrap()
        else {
            panic!("committed")
        };
        assert_eq!(
            status.phase,
            ReceiptPhase::Committed {
                sequence: EventSeq(1)
            }
        );
        assert_eq!(status.operation, fixture.original.operation());
        assert_eq!(status.binding, fixture.original.binding_digest());
        assert_eq!(fixture.client.book_anchors()[0].generation, 2);
        assert_eq!(
            std::fs::read(fixture.scratch.root.join("server.sqlite")).unwrap(),
            committed_database
        );
    }
}

#[test]
fn actual_anchored_book_reopen_rejects_changed_ruleset_and_content() {
    let fixture = fixture::Fixture::new();
    let anchors = fixture.client.book_anchors().to_vec();
    let config = fixture.client.config();
    let fixture::Fixture {
        server,
        client,
        scratch,
        ..
    } = fixture;
    drop(client);
    drop(server);
    for ruleset in [true, false] {
        let mut changed = config;
        if ruleset {
            changed.ruleset[0] ^= 1;
        } else {
            changed.content[0] ^= 1;
        }
        let store = nf_store::Store::open_existing(
            scratch.root.join("client.sqlite"),
            nf_store::KnownFrontiers::genesis(config.scope.universe, config.scope.history),
        )
        .unwrap();
        let vault = nf_identity::private_storage::PrivateVault::open(
            &scratch.root.join("client-private"),
            &scratch.root.join("saves"),
        )
        .unwrap();
        assert!(
            matches!(
                ReceiptRepo::open_owned(store, vault, changed, &anchors),
                Err(nf_transport::PeerError::Scope)
            ),
            "anchored immutable context must match repository"
        );
    }
}

#[test]
fn actual_sql_world_context_rejects_a_different_configured_ruleset_without_book() {
    let fixture = fixture::Fixture::new();
    let mut config = fixture.server.config();
    config.ruleset[0] ^= 1;
    let fixture::Fixture {
        server,
        client,
        scratch,
        ..
    } = fixture;
    drop(server);
    drop(client);
    let store = nf_store::Store::open_existing(
        scratch.root.join("server.sqlite"),
        nf_store::KnownFrontiers::genesis(config.scope.universe, config.scope.history),
    )
    .unwrap();
    let vault = nf_identity::private_storage::PrivateVault::open(
        &scratch.root.join("server-private"),
        &scratch.root.join("saves"),
    )
    .unwrap();
    assert!(
        matches!(
            ReceiptRepo::open_owned(store, vault, config, &[]),
            Err(nf_transport::PeerError::Scope)
        ),
        "actual SQL world ruleset must match configured context"
    );
}

#[test]
fn legitimate_opaque_reply_cannot_be_persisted_under_another_static_context() {
    let mut f = fixture::Fixture::new();
    let mut other = fork::alternate_client(&f);
    let original_bytes =
        std::fs::read(f.scratch.root.join("alternate-private/blob-receipt-r0-g0")).unwrap();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, minimum) = f.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
    let response = server.prove(proof, cp, id, &mut f.server).unwrap();
    let verified = client.reply(response, sp, id, &mut f.client).unwrap();
    assert!(
        matches!(
            other.accept_verified(0, verified),
            Err(nf_transport::PeerError::Scope)
        ),
        "proof from A must carry its static context into final B admission"
    );
    assert_eq!(other.book_anchors()[0].generation, 0);
    assert_eq!(
        std::fs::read(f.scratch.root.join("alternate-private/blob-receipt-r0-g0")).unwrap(),
        original_bytes
    );
    assert!(
        !f.scratch
            .root
            .join("alternate-private/blob-receipt-r0-g1")
            .exists()
    );
}

#[test]
fn first_wrong_peer_connection_and_signed_prefix_attempts_are_consumed() {
    let mut f = fixture::Fixture::new();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    for case in 0..4 {
        let id = libp2p::swarm::ConnectionId::new_unchecked(case + 1);
        let wrong = libp2p::swarm::ConnectionId::new_unchecked(case + 10);
        let (cs, ss) = f.sessions(id);
        let (original, minimum) = f.client.original_for_slot(0).unwrap();
        let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
        let mut server = ReceiptOperationServer::new(ss).unwrap();
        let begin = client.begin(&mut f.client).unwrap();
        let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
        if case == 0 {
            assert!(
                client
                    .challenge(challenge.clone(), cp, id, &mut f.client)
                    .is_err()
            );
            assert!(matches!(
                client.challenge(challenge, sp, id, &mut f.client),
                Err(nf_transport::PeerError::Replay)
            ));
            continue;
        }
        let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
        if case == 1 {
            assert!(
                server
                    .prove(proof.clone(), cp, wrong, &mut f.server)
                    .is_err()
            );
            assert!(matches!(
                server.prove(proof, cp, id, &mut f.server),
                Err(nf_transport::PeerError::Replay)
            ));
            continue;
        }
        if case == 2 {
            let mut modified = proof.clone();
            let nf_transport::receipt::ReceiptBody::Prove { proof: p, .. } = &mut modified.body
            else {
                panic!("proof");
            };
            p.challenge[0] ^= 1;
            assert!(server.prove(modified, cp, id, &mut f.server).is_err());
            assert!(matches!(
                server.prove(proof, cp, id, &mut f.server),
                Err(nf_transport::PeerError::Replay)
            ));
            continue;
        }
        let response = server.prove(proof, cp, id, &mut f.server).unwrap();
        let mut modified = response.clone();
        let nf_transport::receipt::ReceiptBody::Status { status, .. } = &mut modified.body else {
            panic!("status");
        };
        status.current.store_revision += 1;
        assert!(client.reply(modified, sp, id, &mut f.client).is_err());
        assert!(matches!(
            client.reply(response, sp, id, &mut f.client),
            Err(nf_transport::PeerError::Replay)
        ));
    }
    assert_eq!(f.client.book_anchors()[0].generation, 0);
}

#[test]
fn actual_signed_durable_revocation_fences_pending_server_and_client_attempts() {
    for server_cut in [true, false] {
        let mut f = fixture::Fixture::new();
        let id = libp2p::swarm::ConnectionId::new_unchecked(1);
        let (cs, ss) = f.sessions(id);
        let (original, minimum) = f.client.original_for_slot(0).unwrap();
        let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
        let mut server = ReceiptOperationServer::new(ss).unwrap();
        let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
        let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
        let begin = client.begin(&mut f.client).unwrap();
        let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
        let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
        let change = nf_identity::model::DeviceRevocation {
            scope: f.state.scope,
            issuer: f.server_public.account,
            device: f.client_public.device,
            frontier: f.state.revision,
        };
        let signature = f
            .owner_key
            .sign(&nf_identity::rotation::revocation_digest(&change));
        if server_cut {
            assert_eq!(
                f.server.revoke_trusted(change, signature).unwrap(),
                f.state.revision + 1
            );
            assert!(server.prove(proof.clone(), cp, id, &mut f.server).is_err());
            assert!(matches!(
                server.prove(proof, cp, id, &mut f.server),
                Err(nf_transport::PeerError::Replay)
            ));
        } else {
            let response = server.prove(proof, cp, id, &mut f.server).unwrap();
            assert_eq!(
                f.client.revoke_trusted(change, signature).unwrap(),
                f.state.revision + 1
            );
            assert!(
                client
                    .reply(response.clone(), sp, id, &mut f.client)
                    .is_err()
            );
            assert!(matches!(
                client.reply(response, sp, id, &mut f.client),
                Err(nf_transport::PeerError::Replay)
            ));
        }
        assert_eq!(f.client.book_anchors()[0].generation, 0);
    }
}

#[test]
fn below_protected_source_minima_returns_only_signed_unsupported() {
    let mut f = fixture::Fixture::new();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, mut minimum) = f.client.original_for_slot(0).unwrap();
    minimum.event = EventSeq(1);
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
    let response = server.prove(proof, cp, id, &mut f.server).unwrap();
    let verified = client.reply(response, sp, id, &mut f.client).unwrap();
    assert!(matches!(
        f.client.accept_verified(0, verified),
        Ok(AdmittedReceipt::Unsupported(
            nf_transport::receipt::ReceiptUnsupportedReason::SourceBelowKnownMinima
        ))
    ));
    assert_eq!(f.client.book_anchors()[0].generation, 0);
}

#[test]
fn lost_committed_reply_reopens_same_original_and_stale_session_reply_is_refused() {
    let mut f = fixture::Fixture::new();
    let ack = f
        .server
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, minimum) = f.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(cs, id, original.clone(), minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
    let lost = server.prove(proof, cp, id, &mut f.server).unwrap();
    drop(client);
    drop(server);
    let sc = f.server.config();
    let cc = f.client.config();
    let anchors = f.client.book_anchors().to_vec();
    let fixture::Fixture {
        server,
        client,
        scratch,
        owner_key,
        server_public,
        client_public,
        state,
        ..
    } = f;
    drop(server);
    drop(client);
    let saved_database = std::fs::read(scratch.root.join("server.sqlite")).unwrap();
    let source = nf_store::Store::open_existing(
        scratch.root.join("server.sqlite"),
        nf_store::KnownFrontiers {
            scope: sc.scope,
            event_sequence: ack.sequence(),
            store_revision: ack.revision(),
            membership_revision: Some(state.revision),
        },
    )
    .unwrap();
    assert_eq!(source.world().to_spec().markets[0].credits, 95);
    let local = nf_store::Store::open_existing(
        scratch.root.join("client.sqlite"),
        nf_store::KnownFrontiers {
            scope: cc.scope,
            event_sequence: EventSeq(0),
            store_revision: 0,
            membership_revision: Some(state.revision),
        },
    )
    .unwrap();
    let sv = nf_identity::private_storage::PrivateVault::open(
        &scratch.root.join("server-private"),
        &scratch.root.join("saves"),
    )
    .unwrap();
    let cv = nf_identity::private_storage::PrivateVault::open(
        &scratch.root.join("client-private"),
        &scratch.root.join("saves"),
    )
    .unwrap();
    let mut f = fixture::Fixture {
        server: ReceiptRepo::open_owned(source, sv, sc, &[]).unwrap(),
        client: ReceiptRepo::open_owned(local, cv, cc, &anchors).unwrap(),
        owner_key,
        server_public,
        client_public,
        original: original.clone(),
        commit: None,
        state,
        scratch,
    };
    assert_eq!(
        f.client.original_for_slot(0).unwrap().0.original(),
        original.original()
    );
    for attempt in 0..3 {
        let id = libp2p::swarm::ConnectionId::new_unchecked(attempt + 2);
        let (cs, ss) = f.sessions(id);
        let (original, minimum) = f.client.original_for_slot(0).unwrap();
        let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
        let mut server = ReceiptOperationServer::new(ss).unwrap();
        let begin = client.begin(&mut f.client).unwrap();
        let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
        let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
        let response = server.prove(proof, cp, id, &mut f.server).unwrap();
        if attempt == 0 {
            assert!(client.reply(lost.clone(), sp, id, &mut f.client).is_err());
            assert!(matches!(
                client.reply(response, sp, id, &mut f.client),
                Err(nf_transport::PeerError::Replay)
            ));
        } else {
            let verified = client.reply(response, sp, id, &mut f.client).unwrap();
            let AdmittedReceipt::Status(status) = f.client.accept_verified(0, verified).unwrap()
            else {
                panic!("receipt");
            };
            assert_eq!(
                status.phase,
                ReceiptPhase::Committed {
                    sequence: EventSeq(1)
                }
            );
            assert_eq!(f.client.book_anchors()[0].generation, 1);
        }
    }
    assert_eq!(
        std::fs::read(f.scratch.root.join("server.sqlite")).unwrap(),
        saved_database
    );
}

#[test]
fn actually_signed_terminal_above_current_event_and_changed_binding_are_refused() {
    use nf_transport::receipt::{ReceiptBody, ReceiptOperation, reply_prefix_digest};
    let mut f = fixture::Fixture::new();
    let vault = nf_identity::private_storage::PrivateVault::open(
        &f.scratch.root.join("server-private"),
        &f.scratch.root.join("saves"),
    )
    .unwrap();
    let signer = vault.load_identity(f.server_public.peer.clone()).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    for terminal in [true, false] {
        let id = libp2p::swarm::ConnectionId::new_unchecked(if terminal { 1 } else { 2 });
        let (cs, ss) = f.sessions(id);
        let digest = cs.handshake().context_digest().unwrap();
        let limits = cs.handshake().context().selected;
        let (original, minimum) = f.client.original_for_slot(0).unwrap();
        let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
        let mut server = ReceiptOperationServer::new(ss).unwrap();
        let begin = client.begin(&mut f.client).unwrap();
        let ReceiptBody::Begin {
            target,
            nonce,
            minimum,
        } = begin.body.clone()
        else {
            panic!("begin");
        };
        let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
        let ReceiptBody::Challenge {
            server_nonce,
            frontier,
            ..
        } = challenge.body.clone()
        else {
            panic!("challenge");
        };
        let operation = ReceiptOperation {
            context_digest: digest,
            target,
            client_nonce: nonce,
            server_nonce,
            frontier,
            minimum,
        };
        let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
        let mut response = server.prove(proof, cp, id, &mut f.server).unwrap();
        let ReceiptBody::Status { status, .. } = &mut response.body else {
            panic!("status");
        };
        if terminal {
            status.phase = ReceiptPhase::Committed {
                sequence: EventSeq(1),
            };
        } else {
            status.binding[0] ^= 1;
        }
        let digest = operation
            .challenge(2, reply_prefix_digest(&response, limits).unwrap())
            .unwrap();
        let ReceiptBody::Status { proof, .. } = &mut response.body else {
            panic!("proof");
        };
        proof.challenge = digest;
        let signed = nf_identity::signing::device_digest(proof).unwrap();
        proof.signature = signer.device_key.sign(&signed);
        f.state
            .authorize(
                proof,
                &f.server_public.peer,
                &digest,
                minimum.membership_revision,
                nf_identity::model::ProtectedOperation::Economic,
            )
            .unwrap();
        assert!(matches!(
            client.reply(response.clone(), sp, id, &mut f.client),
            Err(nf_transport::PeerError::Unauthorized)
        ));
        assert!(matches!(
            client.reply(response, sp, id, &mut f.client),
            Err(nf_transport::PeerError::Replay)
        ));
        assert_eq!(f.client.book_anchors()[0].generation, 0);
    }
}

#[test]
fn real_operation_deadline_expires_and_consumes_attempt_without_persistence() {
    let mut f = fixture::Fixture::new();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, minimum) = f.client.original_for_slot(0).unwrap();
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let started = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(5050));
    assert!(started.elapsed() >= std::time::Duration::from_secs(5));
    assert!(matches!(
        client.challenge(challenge.clone(), sp, id, &mut f.client),
        Err(nf_transport::PeerError::Replay)
    ));
    assert!(matches!(
        client.challenge(challenge, sp, id, &mut f.client),
        Err(nf_transport::PeerError::Replay)
    ));
    assert_eq!(f.client.book_anchors()[0].generation, 0);
}

#[test]
fn actual_historical_commit_below_external_minimum_is_admitted_from_fresh_later_source() {
    let mut f = fixture::Fixture::historical();
    let before = std::fs::read(f.scratch.root.join("server.sqlite")).unwrap();
    let id = libp2p::swarm::ConnectionId::new_unchecked(1);
    let (cs, ss) = f.sessions(id);
    let (original, mut minimum) = f.client.original_for_slot(0).unwrap();
    minimum.event = EventSeq(2);
    let mut client = ReceiptOperationClient::new(cs, id, original, minimum).unwrap();
    let mut server = ReceiptOperationServer::new(ss).unwrap();
    let sp = libp2p::PeerId::from_bytes(&f.server_public.peer).unwrap();
    let cp = libp2p::PeerId::from_bytes(&f.client_public.peer).unwrap();
    let begin = client.begin(&mut f.client).unwrap();
    let challenge = server.begin(begin, cp, id, &mut f.server).unwrap();
    let proof = client.challenge(challenge, sp, id, &mut f.client).unwrap();
    let response = server.prove(proof, cp, id, &mut f.server).unwrap();
    let verified = client.reply(response, sp, id, &mut f.client).unwrap();
    let AdmittedReceipt::Status(status) = f.client.accept_verified(0, verified).unwrap() else {
        panic!("receipt");
    };
    assert_eq!(
        status.phase,
        ReceiptPhase::Committed {
            sequence: EventSeq(1)
        }
    );
    assert_eq!(status.current.event, EventSeq(3));
    assert_eq!(f.client.book_anchors()[0].minimum.event, EventSeq(3));
    assert_eq!(
        std::fs::read(f.scratch.root.join("server.sqlite")).unwrap(),
        before
    );
}

#[path = "../receipt_effect_support/lane_cases.rs"]
mod lane_cases;

#[path = "../receipt_effect_support/malformed_cases.rs"]
mod malformed_cases;

#[path = "timing.rs"]
mod timing;

#[path = "timing_origin.rs"]
mod timing_origin;

#[path = "activation.rs"]
mod activation;
