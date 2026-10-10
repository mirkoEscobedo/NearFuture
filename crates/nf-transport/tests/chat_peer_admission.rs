//! Local resource policy oracles over genuine Noise. No wire-supplied identity grants authority.
#[path = "chat_peer_delivery_support/mod.rs"]
mod original_fixture;
use libp2p::Multiaddr;
use original_fixture::Fixture;
mod chat_peer_admission_support;
use chat_peer_admission_support::Raw;
use nf_contract::identity::RequestId;
use nf_identity::private_storage::{LocalIdentity, PrivateVault};
use nf_store::chat::outbox::{ClientOutbox, OutgoingState};
use nf_store::chat::{ChatStore, IssuedChallenge, KnownChatFrontiers, SignedMessage, codec};
use nf_transport::{
    chat::{ChatFrame, ChatPeerServer, ChatPeerServerEvent, Refusal, WireContext},
    identity::TransportIdentity,
};
use std::{
    fs,
    time::{Duration, Instant},
};
const CASE_AGE: Duration = Duration::from_secs(30);

fn context(f: &Fixture, id: u8) -> WireContext {
    WireContext {
        policy_digest: codec::policy_digest(&f.policy),
        request: RequestId::from_bytes([id; 16]),
    }
}
fn post(f: &Fixture, id: u8, signed: &SignedMessage) -> ChatFrame {
    ChatFrame::PostChallenge {
        context: context(f, id),
        signed: Box::new(signed.clone()),
    }
}
fn refused(f: &Fixture, id: u8, reason: Refusal) -> ChatFrame {
    ChatFrame::Refused {
        context: context(f, id),
        reason,
    }
}
async fn issued(
    raw: &mut Raw,
    server: &mut ChatPeerServer,
    f: &Fixture,
    id: u8,
    signed: &SignedMessage,
) -> IssuedChallenge {
    match raw.post(server, f.server_peer, post(f, id, signed)).await {
        ChatFrame::Issued {
            context: actual,
            challenge,
        } => {
            assert_eq!(actual, context(f, id));
            challenge
        }
        other => panic!("independent literal allowance expected Issued, got {other:?}"),
    }
}
fn extra(f: &Fixture, name: &str) -> (PrivateVault, LocalIdentity) {
    let root = f.receiver_path.parent().unwrap();
    let vault = PrivateVault::create(&root.join(name), &root.join("saves")).unwrap();
    let noise = TransportIdentity::create(&vault).unwrap();
    let local = vault.create_identity(noise.peer_id().to_bytes()).unwrap();
    (vault, local)
}
fn second_device(f: &mut Fixture) -> (PrivateVault, SignedMessage) {
    let (vault, local) = extra(f, "second-device-private");
    // TRUSTED LOCAL INITIAL state only: redeem rejects an existing account. This is not enrollment.
    f.state.devices.insert(
        local.public.device,
        nf_identity::model::Device {
            account: f.alice.public.account,
            key: local.public.device_key,
            peer: local.public.peer.clone(),
            revoked: false,
        },
    );
    nf_identity::codec::validate_state(&f.state).unwrap();
    let mut signed = f.signed.clone();
    signed.message.author.device = local.public.device;
    signed.signature = local
        .device_key
        .sign(&codec::message_digest(&signed.message).unwrap());
    (vault, signed)
}
fn server(f: &Fixture) -> (ChatPeerServer, Vec<u8>) {
    assert!(!f.outbox_path.exists());
    let store = ChatStore::create(&f.receiver_path, &f.policy, &f.state).unwrap();
    drop(store);
    let before = fs::read(&f.receiver_path).unwrap();
    let store = ChatStore::open_existing(
        &f.receiver_path,
        &f.policy,
        KnownChatFrontiers {
            scope: f.policy.scope,
            revision: 0,
            membership_revision: f.state.revision,
        },
    )
    .unwrap();
    (
        ChatPeerServer::new(store, &f.server_vault, f.policy).unwrap(),
        before,
    )
}
async fn ready(server: &mut ChatPeerServer, f: &Fixture) -> Multiaddr {
    loop {
        if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
            assert_eq!(peer, f.server_peer);
            return address;
        }
    }
}
fn unchanged(server: ChatPeerServer, f: &Fixture, before: Vec<u8>) {
    let store = server.into_store();
    assert_eq!(store.current_membership(), Ok(f.state.clone()));
    assert_eq!(
        store.known_frontiers(),
        Ok(KnownChatFrontiers {
            scope: f.policy.scope,
            revision: 0,
            membership_revision: f.state.revision
        })
    );
    drop(store);
    assert_eq!(fs::read(&f.receiver_path).unwrap(), before);
}

#[tokio::test(flavor = "current_thread")]
async fn one_account_has_eight_challenges_across_real_devices_and_reconnect() {
    let mut f = Fixture::new();
    assert_eq!(
        TransportIdentity::load(&f.client_vault).unwrap().peer_id(),
        f.client_peer
    );
    let (second_vault, second_signed) = second_device(&mut f);
    let (mut server, before) = server(&f);
    let mut pending = f.expected();
    pending.state = OutgoingState::Pending;
    let mut outbox = ClientOutbox::create(&f.outbox_path, &f.profile).unwrap();
    assert_eq!(outbox.enqueue(f.request(), &f.signed), Ok(pending.clone()));
    drop(outbox);
    let pending_bytes = fs::read(&f.outbox_path).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    tokio::time::timeout(CASE_AGE, async {
        let address = ready(&mut server, &f).await;
        let started = Instant::now();
        let mut first = Raw::new(&f.client_vault);
        first
            .connect(&mut server, address.clone(), f.server_peer)
            .await;
        for id in 101..105 {
            issued(&mut first, &mut server, &f, id, &f.signed).await;
        }
        let original = first.ids.unwrap();
        first.disconnect(&mut server, f.server_peer).await;
        let mut second = Raw::new(&second_vault);
        second
            .connect(&mut server, address.clone(), f.server_peer)
            .await;
        for id in 105..109 {
            issued(&mut second, &mut server, &f, id, &second_signed).await;
        }
        second.disconnect(&mut server, f.server_peer).await;
        first.connect(&mut server, address, f.server_peer).await;
        assert_ne!(original, first.ids.unwrap());
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "setup must remain in the same account window"
        );
        assert_eq!(
            first
                .post(&mut server, f.server_peer, post(&f, 109, &f.signed))
                .await,
            refused(&f, 109, Refusal::Limit)
        );
        first.disconnect(&mut server, f.server_peer).await;
        assert_eq!(outbox.entry([81; 16]), Ok(Some(pending)));
        assert_eq!(outbox.known_revision(), Ok(1));
        drop(outbox);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), pending_bytes);
        unchanged(server, &f, before);
    })
    .await
    .expect("bounded genuine same-account devices/reconnect oracle");
}

// Additive independent controls: the complete qualified first above is preserved byte-for-byte.
use nf_store::chat::{Channel, HistoryQuery};
use nf_transport::chat::history::HistoryFrame;
use original_fixture::author;
fn proof(f: &Fixture, id: u8, challenge: IssuedChallenge) -> ChatFrame {
    let mut proof = nf_identity::model::DeviceProof {
        scope: f.policy.scope,
        account: f.alice.public.account,
        device: f.alice.public.device,
        frontier: challenge.membership_revision,
        peer: f.client_peer.to_bytes(),
        challenge: challenge.challenge,
        signature: [0; 64],
    };
    proof.signature = f
        .alice
        .device_key
        .sign(&nf_identity::signing::device_digest(&proof).unwrap());
    ChatFrame::PostProof {
        context: context(f, id),
        signed: Box::new(f.signed.clone()),
        ticket: challenge.ticket,
        proof: Box::new(proof),
    }
}
fn unchanged_without_outbox(server: ChatPeerServer, f: &Fixture, before: Vec<u8>) {
    unchanged(server, f, before);
    assert!(!f.outbox_path.exists());
}

#[tokio::test(flavor = "current_thread")]
async fn post_and_history_share_eight_but_an_admitted_proof_can_finish() {
    let f = Fixture::new();
    let (mut server, _) = server(&f);
    tokio::time::timeout(CASE_AGE, async {
        let address = ready(&mut server, &f).await;
        let mut raw = Raw::new(&f.client_vault);
        raw.connect(&mut server, address, f.server_peer).await;
        let started = Instant::now();
        let first = issued(&mut raw, &mut server, &f, 101, &f.signed).await;
        for id in 102..105 {
            issued(&mut raw, &mut server, &f, id, &f.signed).await;
        }
        for id in 105..109 {
            let query = HistoryQuery {
                request: context(&f, id).request,
                reader: author(&f.alice),
                channel: Channel::General,
                after_cursor: 0,
                limit: 1,
            };
            match raw
                .history(
                    &mut server,
                    f.server_peer,
                    HistoryFrame::Challenge {
                        context: context(&f, id),
                        query,
                    },
                )
                .await
            {
                HistoryFrame::Issued {
                    context: actual,
                    query: actual_query,
                    ..
                } => {
                    assert_eq!(actual, context(&f, id));
                    assert_eq!(actual_query, query);
                }
                other => panic!("permitted genuine history challenge missing: {other:?}"),
            }
        }
        assert_eq!(
            raw.post(&mut server, f.server_peer, post(&f, 109, &f.signed))
                .await,
            refused(&f, 109, Refusal::Limit)
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "first original proof must still be unexpired"
        );
        let expected = match f.expected().state {
            nf_store::chat::outbox::OutgoingState::Delivered(signed) => signed,
            _ => unreachable!(),
        };
        assert_eq!(
            raw.post(&mut server, f.server_peer, proof(&f, 101, first))
                .await,
            ChatFrame::Delivered {
                context: context(&f, 101),
                signed: expected
            }
        );
        raw.disconnect(&mut server, f.server_peer).await;
        let store = server.into_store();
        assert_eq!(
            store.known_frontiers(),
            Ok(KnownChatFrontiers {
                scope: f.policy.scope,
                revision: 1,
                membership_revision: 1
            })
        );
        drop(store);
    })
    .await
    .expect("bounded shared quota with original five-second proof authority");
}
#[tokio::test(flavor = "current_thread")]
async fn unknown_foreign_revoked_and_mismatched_devices_never_charge_a_claimed_account() {
    for attack in 0..4 {
        let mut f = Fixture::new();
        let (attacker_vault, local) = extra(&f, "attacker-private");
        let mut signed = f.signed.clone();
        // Trusted LOCAL initial state, not a claimed enrollment/rotation transition.
        if attack == 1 {
            f.state.accounts.insert(
                local.public.account,
                nf_identity::model::Account {
                    key: local.public.account_key,
                    roles: nf_identity::model::Roles::PLAYER,
                },
            );
            f.state.devices.insert(
                local.public.device,
                nf_identity::model::Device {
                    account: local.public.account,
                    key: local.public.device_key,
                    peer: local.public.peer.clone(),
                    revoked: false,
                },
            );
        } else if attack >= 2 {
            f.state.devices.insert(
                local.public.device,
                nf_identity::model::Device {
                    account: f.alice.public.account,
                    key: local.public.device_key,
                    peer: local.public.peer.clone(),
                    revoked: attack == 2,
                },
            );
            if attack == 2 {
                signed.message.author.device = local.public.device;
                signed.signature = local
                    .device_key
                    .sign(&codec::message_digest(&signed.message).unwrap());
            }
        }
        nf_identity::codec::validate_state(&f.state).unwrap();
        let attacker = &attacker_vault;
        let (mut server, before) = server(&f);
        tokio::time::timeout(CASE_AGE, async {
            let address = ready(&mut server, &f).await;
            let mut bad = Raw::new(attacker);
            bad.connect(&mut server, address.clone(), f.server_peer)
                .await;
            bad.refused_and_closed(
                &mut server,
                f.server_peer,
                post(&f, 100, &signed),
                refused(&f, 100, Refusal::Unauthorized),
            )
            .await;
            let mut alice = Raw::new(&f.client_vault);
            alice.connect(&mut server, address, f.server_peer).await;
            let started = Instant::now();
            for id in 101..109 {
                issued(&mut alice, &mut server, &f, id, &f.signed).await;
            }
            assert!(started.elapsed() < Duration::from_secs(30));
            assert_eq!(
                alice
                    .post(&mut server, f.server_peer, post(&f, 109, &f.signed))
                    .await,
                refused(&f, 109, Refusal::Limit)
            );
            alice.disconnect(&mut server, f.server_peer).await;
            unchanged_without_outbox(server, &f, before);
        })
        .await
        .expect("bounded untrusted actor attribution");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn a_noise_peer_bound_to_two_current_accounts_is_not_chargeable() {
    for roles in [
        nf_identity::model::Roles::PLAYER,
        nf_identity::model::Roles::REPLICA,
    ] {
        let mut f = Fixture::new();
        let (_vault, local) = extra(&f, "ambiguous-private");
        // Valid trusted LOCAL initial state: codec does not impose peer uniqueness across accounts.
        f.state.accounts.insert(
            local.public.account,
            nf_identity::model::Account {
                key: local.public.account_key,
                roles,
            },
        );
        f.state.devices.insert(
            local.public.device,
            nf_identity::model::Device {
                account: local.public.account,
                key: local.public.device_key,
                peer: f.client_peer.to_bytes(),
                revoked: false,
            },
        );
        nf_identity::codec::validate_state(&f.state).unwrap();
        let (mut server, before) = server(&f);
        tokio::time::timeout(CASE_AGE, async {
            let address = ready(&mut server, &f).await;
            let mut alice = Raw::new(&f.client_vault);
            alice.connect(&mut server, address, f.server_peer).await;
            alice
                .refused_and_closed(
                    &mut server,
                    f.server_peer,
                    post(&f, 101, &f.signed),
                    refused(&f, 101, Refusal::Unauthorized),
                )
                .await;
            unchanged_without_outbox(server, &f, before);
        })
        .await
        .expect("bounded ambiguous physical binding refusal");
    }
}
#[tokio::test(flavor = "current_thread")]
async fn a_revoked_other_account_with_the_same_noise_key_cannot_charge_an_active_player() {
    let mut f = Fixture::new();
    let (_vault, local) = extra(&f, "revoked-shared-peer-private");
    // TRUSTED LOCAL INITIAL state: historical B retains Alice's full peer bytes after revocation.
    // This is not a signed enrollment/rotation transition or proof of a second key holder in a game.
    f.state.accounts.insert(
        local.public.account,
        nf_identity::model::Account {
            key: local.public.account_key,
            roles: nf_identity::model::Roles::PLAYER,
        },
    );
    f.state.devices.insert(
        local.public.device,
        nf_identity::model::Device {
            account: local.public.account,
            key: local.public.device_key,
            peer: f.client_peer.to_bytes(),
            revoked: true,
        },
    );
    nf_identity::codec::validate_state(&f.state).unwrap();
    let (mut server, before) = server(&f);
    tokio::time::timeout(CASE_AGE, async {
        let address = ready(&mut server, &f).await;
        let mut alice = Raw::new(&f.client_vault);
        alice.connect(&mut server, address, f.server_peer).await;
        alice
            .refused_and_closed(
                &mut server,
                f.server_peer,
                post(&f, 101, &f.signed),
                refused(&f, 101, Refusal::Unauthorized),
            )
            .await;
        unchanged_without_outbox(server, &f, before);
    })
    .await
    .expect("bounded historical shared-key attribution refusal");
}
fn replay(f: &Fixture) -> ChatFrame {
    proof(
        f,
        101,
        IssuedChallenge {
            ticket: [1; 16],
            challenge: [0; 32],
            membership_revision: 1,
        },
    )
}
#[tokio::test(flavor = "current_thread")]
async fn a_sixty_fifth_physical_frame_returns_limit_and_closes() {
    let f = Fixture::new();
    let (mut server, before) = server(&f);
    tokio::time::timeout(CASE_AGE, async {
        let started = Instant::now();
        let address = ready(&mut server, &f).await;
        let mut raw = Raw::new(&f.client_vault);
        raw.connect(&mut server, address, f.server_peer).await;
        for _ in 0..64 {
            assert_eq!(
                raw.post(&mut server, f.server_peer, replay(&f)).await,
                refused(&f, 101, Refusal::Replay)
            );
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "physical-frame probe must remain in its original window"
        );
        raw.refused_and_closed(
            &mut server,
            f.server_peer,
            replay(&f),
            refused(&f, 101, Refusal::Limit),
        )
        .await;
        unchanged_without_outbox(server, &f, before);
    })
    .await
    .expect("bounded actual physical replay pressure");
}
#[tokio::test(flavor = "current_thread")]
async fn reconnect_cannot_reset_the_global_pre_membership_frame_budget() {
    let f = Fixture::new();
    let (mut server, before) = server(&f);
    tokio::time::timeout(CASE_AGE, async {
        let started = Instant::now();
        let address = ready(&mut server, &f).await;
        let mut raw = Raw::new(&f.client_vault);
        for _ in 0..2 {
            raw.connect(&mut server, address.clone(), f.server_peer)
                .await;
            for _ in 0..64 {
                assert_eq!(
                    raw.post(&mut server, f.server_peer, replay(&f)).await,
                    refused(&f, 101, Refusal::Replay)
                );
            }
            raw.disconnect(&mut server, f.server_peer).await;
        }
        raw.connect(&mut server, address, f.server_peer).await;
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "global-frame probe must remain in its original window"
        );
        raw.refused_and_closed(
            &mut server,
            f.server_peer,
            replay(&f),
            refused(&f, 101, Refusal::Limit),
        )
        .await;
        unchanged_without_outbox(server, &f, before);
    })
    .await
    .expect("bounded genuine reconnect flood");
}

#[tokio::test(flavor = "current_thread")]
async fn exhausting_alice_does_not_spend_a_distinct_authenticated_players_allowance() {
    let mut f = Fixture::new();
    let (third_vault, third) = extra(&f, "independent-player-private");
    // TRUSTED LOCAL INITIAL membership for a real, separately persisted Noise/account/device key.
    // No enrollment or rotation claim; the signed proof/store authorities stay unchanged.
    f.state.accounts.insert(
        third.public.account,
        nf_identity::model::Account {
            key: third.public.account_key,
            roles: nf_identity::model::Roles::PLAYER,
        },
    );
    f.state.devices.insert(
        third.public.device,
        nf_identity::model::Device {
            account: third.public.account,
            key: third.public.device_key,
            peer: third.public.peer.clone(),
            revoked: false,
        },
    );
    nf_identity::codec::validate_state(&f.state).unwrap();
    let mut third_signed = f.signed.clone();
    third_signed.message.author = author(&third);
    third_signed.signature = third
        .device_key
        .sign(&codec::message_digest(&third_signed.message).unwrap());
    let (mut server, before) = server(&f);
    tokio::time::timeout(CASE_AGE, async {
        let address = ready(&mut server, &f).await;
        let started = Instant::now();
        let mut alice = Raw::new(&f.client_vault);
        alice
            .connect(&mut server, address.clone(), f.server_peer)
            .await;
        for id in 101..109 {
            issued(&mut alice, &mut server, &f, id, &f.signed).await;
        }
        assert_eq!(
            alice
                .post(&mut server, f.server_peer, post(&f, 109, &f.signed))
                .await,
            refused(&f, 109, Refusal::Limit)
        );
        alice.disconnect(&mut server, f.server_peer).await;
        let mut independent = Raw::new(&third_vault);
        independent
            .connect(&mut server, address, f.server_peer)
            .await;
        for id in 201..209 {
            issued(&mut independent, &mut server, &f, id, &third_signed).await;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "both actual account allowances must share the original window"
        );
        assert_eq!(
            independent
                .post(&mut server, f.server_peer, post(&f, 209, &third_signed))
                .await,
            refused(&f, 209, Refusal::Limit)
        );
        independent.disconnect(&mut server, f.server_peer).await;
        unchanged_without_outbox(server, &f, before);
    })
    .await
    .expect("bounded independent-account resource isolation over real Noise");
}
