mod chat_peer_controls_support;
#[path = "chat_peer_delivery_support/mod.rs"]
mod original_fixture;
use chat_peer_controls_support::{
    BadReply, Prepared, RawPeer, context, deliver, issued, malicious, post_frame, ready, unknown,
    wrong_noise,
};
use nf_transport::{
    PeerError,
    chat::{ChatFrame, Refusal},
};
use original_fixture::Fixture;
use std::time::{Duration, Instant};
const CASE_AGE: Duration = Duration::from_secs(30);
#[tokio::test(flavor = "current_thread")]
async fn unknown_genuinely_signed_noise_sender_cannot_post_or_change_pending() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    let (vault, signed) = unknown(&f);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&vault);
        raw.connect(&mut server, address, f.server_peer).await;
        assert_eq!(
            raw.exchange(
                &mut server,
                f.server_peer,
                ChatFrame::PostChallenge {
                    context: context(&f),
                    signed: Box::new(signed)
                }
            )
            .await,
            ChatFrame::Refused {
                context: context(&f),
                reason: Refusal::Unauthorized
            }
        );
        drop(raw);
        state.finish(server.into_store(), &f, 0);
    })
    .await
    .expect("bounded unknown sender control");
}
#[tokio::test(flavor = "current_thread")]
async fn fresh_stored_sender_revocation_refuses_a_stale_local_client_selection() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, true);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        assert_eq!(
            deliver(&mut server, &mut state.outbox, &f, address).await,
            Err(PeerError::Unauthorized)
        );
        state.finish(server.into_store(), &f, 0);
    })
    .await
    .expect("bounded revoked sender control");
}
#[tokio::test(flavor = "current_thread")]
async fn an_actual_foreign_noise_key_at_a_trusted_peer_address_never_receives_chat() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        assert_eq!(
            wrong_noise(&mut state.outbox, &f).await,
            Err(PeerError::Offline)
        );
        let receiver = state.receiver.take().unwrap();
        state.finish(receiver, &f, 0);
    })
    .await
    .expect("bounded wrong actual Noise key control");
}
#[tokio::test(flavor = "current_thread")]
async fn wrong_original_request_or_policy_correlation_keeps_the_original_pending() {
    for bad in [BadReply::Request, BadReply::Policy] {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, false);
        tokio::time::timeout(CASE_AGE, async {
            let mut receiver = state.receiver.take().unwrap();
            assert_eq!(
                malicious(&mut receiver, &mut state.outbox, &f, bad).await,
                Err(PeerError::Session)
            );
            state.finish(receiver, &f, 0);
        })
        .await
        .expect("bounded malformed response correlation control");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn forged_signature_and_valid_bob_signature_over_a_wrong_original_digest_keep_pending() {
    for bad in [BadReply::Signature, BadReply::OriginalDigest] {
        let f = Fixture::new();
        let mut state = Prepared::new(&f, false);
        tokio::time::timeout(CASE_AGE, async {
            let mut receiver = state.receiver.take().unwrap();
            assert_eq!(
                malicious(&mut receiver, &mut state.outbox, &f, bad).await,
                Err(PeerError::Unauthorized)
            );
            state.finish(receiver, &f, 0);
        })
        .await
        .expect("bounded untrusted receipt authority control");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn a_consumed_real_ticket_replays_without_a_second_history_entry_or_ack() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&f.client_vault);
        raw.connect(&mut server, address, f.server_peer).await;
        let started = Instant::now();
        let challenge = issued(&mut raw, &mut server, &f).await;
        let frame = post_frame(&f, challenge);
        let expected = match f.expected().state {
            nf_store::chat::outbox::OutgoingState::Delivered(signed) => signed,
            _ => unreachable!(),
        };
        assert_eq!(
            raw.exchange(&mut server, f.server_peer, frame.clone())
                .await,
            ChatFrame::Delivered {
                context: context(&f),
                signed: expected
            }
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "setup must retain a nonexpired replay ticket"
        );
        assert_eq!(
            raw.exchange(&mut server, f.server_peer, frame).await,
            ChatFrame::Refused {
                context: context(&f),
                reason: Refusal::Replay
            }
        );
        drop(raw);
        state.finish(server.into_store(), &f, 1);
    })
    .await
    .expect("bounded real ticket replay control");
}
#[tokio::test(flavor = "current_thread")]
async fn a_new_actual_connection_cannot_reuse_a_nonexpired_prior_connection_ticket() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&f.client_vault);
        let before = raw
            .connect(&mut server, address.clone(), f.server_peer)
            .await;
        let started = Instant::now();
        let challenge = issued(&mut raw, &mut server, &f).await;
        raw.disconnect(&mut server, f.server_peer).await;
        let after = raw.connect(&mut server, address, f.server_peer).await;
        assert_ne!(before.0, after.0);
        assert_ne!(before.1, after.1);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "setup must retain a nonexpired old connection ticket"
        );
        assert_eq!(
            raw.exchange(&mut server, f.server_peer, post_frame(&f, challenge))
                .await,
            ChatFrame::Refused {
                context: context(&f),
                reason: Refusal::Replay
            }
        );
        drop(raw);
        state.finish(server.into_store(), &f, 0);
    })
    .await
    .expect("bounded actual reconnect ticket control");
}
#[tokio::test(flavor = "current_thread")]
async fn the_original_real_five_second_ticket_age_is_not_extended_by_foreground_polling() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&f.client_vault);
        raw.connect(&mut server, address, f.server_peer).await;
        let challenge = issued(&mut raw, &mut server, &f).await;
        tokio::time::sleep(Duration::from_millis(5050)).await;
        assert_eq!(
            raw.exchange(&mut server, f.server_peer, post_frame(&f, challenge))
                .await,
            ChatFrame::Refused {
                context: context(&f),
                reason: Refusal::Replay
            }
        );
        drop(raw);
        state.finish(server.into_store(), &f, 0);
    })
    .await
    .expect("bounded actual unchanged ticket expiry control");
}
#[tokio::test(flavor = "current_thread")]
async fn second_noise_connection_pressure_leaves_the_first_owner_session_usable() {
    let f = Fixture::new();
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut first = RawPeer::new(&f.client_vault);
        first
            .connect(&mut server, address.clone(), f.server_peer)
            .await;
        let mut second = RawPeer::new(&f.client_vault);
        second
            .rejected_second(&mut server, &mut first, address)
            .await;
        drop(second);
        let challenge = issued(&mut first, &mut server, &f).await;
        assert_eq!(challenge.membership_revision, 1);
        drop(first);
        state.finish(server.into_store(), &f, 0);
    })
    .await
    .expect("bounded physical connection pressure control");
}

/// Observed authentic production receipt interrupted before LOCAL outbox acknowledgment.
/// This deliberately does not claim that an unseen wire response was dropped.
#[tokio::test(flavor = "current_thread")]
async fn observed_production_receipt_lost_before_local_ack_retries_the_same_original_once() {
    use nf_store::chat::{
        ChatStore, KnownChatFrontiers,
        outbox::{ClientOutbox, OutgoingState},
    };
    use nf_transport::{
        chat::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, deliver_pending},
        identity::TransportIdentity,
    };
    use std::fs;
    let f = Fixture::new();
    let expected = f.expected();
    let mut pending = expected.clone();
    pending.state = OutgoingState::Pending;
    let mut state = Prepared::new(&f, false);
    tokio::time::timeout(CASE_AGE, async {
        let mut server = state.server(&f);
        let address = ready(&mut server, &f).await;
        let mut raw = RawPeer::new(&f.client_vault);
        let original_connections = raw.connect(&mut server, address, f.server_peer).await;
        let issued = issued(&mut raw, &mut server, &f).await;
        let expected_receipt = match expected.state.clone() {
            OutgoingState::Delivered(signed) => signed,
            _ => unreachable!(),
        };
        assert_eq!(
            raw.exchange(&mut server, f.server_peer, post_frame(&f, issued)).await,
            ChatFrame::Delivered { context: context(&f), signed: expected_receipt }
        );
        // A real foreground owner posted/signed the original. End the actual client Swarm
        // after its response correlation check, without ever calling outbox.acknowledge.
        drop(raw);
        loop {
            if let ChatPeerServerEvent::Disconnected { peer, connection } = server.next().await.unwrap() {
                assert_eq!(peer, f.client_peer);
                assert_eq!(connection, original_connections.1);
                break;
            }
        }
        let mut receiver = server.into_store();
        let known = KnownChatFrontiers { scope: f.policy.scope, revision: 1, membership_revision: 1 };
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(receiver.current_membership(), Ok(f.state.clone()));
        assert_eq!(state.outbox.entry([81; 16]), Ok(Some(pending.clone())));
        assert_eq!(state.outbox.known_revision(), Ok(1));
        assert_eq!(retained_original_history(&mut receiver, &f), one_original_page(&f));
        drop(receiver);
        drop(state.outbox);
        let posted_receiver_bytes = fs::read(&f.receiver_path).unwrap();
        let pending_outbox_bytes = fs::read(&f.outbox_path).unwrap();
        // Actual closed stores and identities reopen; no fixture SQL/receipt injection.
        let receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(pending)));
        assert_eq!(outbox.known_revision(), Ok(1));
        assert_eq!(TransportIdentity::load(&f.client_vault).unwrap().peer_id(), f.client_peer);
        assert_eq!(TransportIdentity::load(&f.server_vault).unwrap().peer_id(), f.server_peer);
        let mut server = ChatPeerServer::new(receiver, &f.server_vault, f.policy).unwrap();
        let address = ready(&mut server, &f).await;
        let pin = ChatPeerPin::from_current(&f.state, original_fixture::author(&f.bob)).unwrap();
        let mut retry_connection = None;
        let result = {
            let delivery = deliver_pending(&mut outbox, &f.client_vault, &f.state, &pin, address, [81; 16]);
            tokio::pin!(delivery);
            loop {
                tokio::select! {
                    result = &mut delivery => break result,
                    event = server.next() => if let ChatPeerServerEvent::Connected { peer, connection } = event.unwrap() {
                        assert_eq!(peer, f.client_peer);
                        assert!(retry_connection.replace(connection).is_none());
                    }
                }
            }
        };
        assert_eq!(result, Ok(expected.clone()));
        assert_ne!(retry_connection.unwrap(), original_connections.1);
        assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
        assert_eq!(outbox.known_revision(), Ok(2));
        let mut receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(retained_original_history(&mut receiver, &f), one_original_page(&f));
        drop(receiver);
        drop(outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), posted_receiver_bytes);
        let delivered_outbox_bytes = fs::read(&f.outbox_path).unwrap();
        assert_ne!(delivered_outbox_bytes, pending_outbox_bytes);
        let mut receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(retained_original_history(&mut receiver, &f), one_original_page(&f));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(expected)));
        assert_eq!(outbox.known_revision(), Ok(2));
        drop(receiver);
        drop(outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), posted_receiver_bytes);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), delivered_outbox_bytes);
    }).await.expect("bounded real receipt loss before local acknowledgment");
}
fn retained_original_history(
    receiver: &mut nf_store::chat::ChatStore,
    f: &Fixture,
) -> nf_store::chat::HistoryPage {
    use nf_contract::identity::RequestId;
    use nf_store::chat::{ChallengeRequest, HistoryQuery, ProofAttempt};
    let query = HistoryQuery {
        request: RequestId::from_bytes([111; 16]),
        reader: original_fixture::author(&f.bob),
        channel: nf_store::chat::Channel::General,
        after_cursor: 0,
        limit: 2,
    };
    let issued = receiver
        .issue_challenge(ChallengeRequest::History(&query), &f.bob.public.peer)
        .unwrap();
    let proof = chat_peer_controls_support::proof(&f.bob, f.policy.scope, issued);
    receiver
        .history(
            &query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &f.bob.public.peer,
            },
        )
        .unwrap()
}
fn one_original_page(f: &Fixture) -> nf_store::chat::HistoryPage {
    nf_store::chat::HistoryPage {
        entries: vec![nf_store::chat::HistoryEntry {
            receiver_cursor: 1,
            signed: f.signed.clone(),
        }],
        next_cursor: 1,
    }
}
