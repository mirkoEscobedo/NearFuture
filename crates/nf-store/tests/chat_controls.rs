mod chat_controls_support;
mod chat_support;
use chat_controls_support::{
    Snapshot, history, identity, issue_post, post, proof, query, signed, worker,
};
use chat_support::{Fixture, author};
use nf_contract::identity::{HistoryId, RequestId};
use nf_identity::model::IdentityError;
use nf_store::chat::{
    ChallengeRequest, ChatReceipt, ChatStore, ChatStoreError, HistoryEntry, HistoryPage,
    KnownChatFrontiers, ProofAttempt, codec,
};
use std::time::{Duration, Instant};

#[test]
fn fresh_authorization_consumes_failed_tickets_and_denies_non_players() {
    let fixture = Fixture::new();
    let (worker, membership) = worker(&fixture);
    let mut store = ChatStore::create(fixture.database(), &fixture.policy, &membership).unwrap();
    let message = fixture.signed_message();
    let receipt = fixture.post(&mut store, 101, &message).unwrap();
    assert_eq!(receipt.original_request, RequestId::from_bytes([101; 16]));
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.membership_revision, 2);
    let before = Snapshot::take(&fixture);
    let issued = issue_post(&mut store, &fixture.alice, 110, &message).unwrap();
    let mut wrong_scope = fixture.policy.scope;
    wrong_scope.history = HistoryId::from_bytes([9; 16]);
    let invalid = proof(&fixture.alice, wrong_scope, issued);
    assert_eq!(
        store.post(
            RequestId::from_bytes([110; 16]),
            &message,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &invalid,
                peer: &fixture.alice.public.peer,
            }
        ),
        Err(ChatStoreError::Identity(IdentityError::Scope))
    );
    assert_eq!(
        store.post(
            RequestId::from_bytes([110; 16]),
            &message,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &invalid,
                peer: &fixture.alice.public.peer,
            }
        ),
        Err(ChatStoreError::Replay)
    );
    let bob_query = query(&fixture.bob, 111, 0, 64);
    let issued = store
        .issue_challenge(
            ChallengeRequest::History(&bob_query),
            &fixture.bob.public.peer,
        )
        .unwrap();
    let wrong_actor = proof(&fixture.alice, fixture.policy.scope, issued);
    assert_eq!(
        store.history(
            &bob_query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &wrong_actor,
                peer: &fixture.bob.public.peer,
            }
        ),
        Err(ChatStoreError::Signature)
    );
    assert_eq!(
        store.history(
            &bob_query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &wrong_actor,
                peer: &fixture.bob.public.peer,
            }
        ),
        Err(ChatStoreError::Replay)
    );
    let unknown = identity(b"fixture-chat-controls-unknown");
    assert_eq!(
        store.issue_challenge(
            ChallengeRequest::History(&query(&unknown, 112, 0, 64)),
            &unknown.public.peer
        ),
        Err(ChatStoreError::Identity(IdentityError::UnknownDevice))
    );
    assert_eq!(
        store.issue_challenge(
            ChallengeRequest::History(&query(&worker, 113, 0, 64)),
            &worker.public.peer
        ),
        Err(ChatStoreError::Identity(IdentityError::RolePolicy))
    );
    let worker_message = signed(&fixture, &worker, 84, 1, "signed worker-only refusal");
    assert_eq!(
        issue_post(&mut store, &worker, 114, &worker_message),
        Err(ChatStoreError::Identity(IdentityError::RolePolicy))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    before.assert_unchanged(&fixture);
    assert_eq!(
        fixture.permitted_history(&mut store, 115).unwrap().entries,
        vec![HistoryEntry {
            receiver_cursor: 1,
            signed: message
        }]
    );
    before.assert_unchanged(&fixture);
}

#[test]
fn global_request_message_and_source_position_reuse_conflict_without_effects() {
    let fixture = Fixture::new();
    let mut store =
        ChatStore::create(fixture.database(), &fixture.policy, &fixture.membership).unwrap();
    let original = fixture.signed_message();
    let expected = fixture.post(&mut store, 101, &original).unwrap();
    assert_eq!(fixture.post(&mut store, 102, &original), Ok(expected));
    let known = store.known_frontiers().unwrap();
    let before = Snapshot::take(&fixture);
    let changed_request = signed(&fixture, &fixture.alice, 82, 2, "different request payload");
    assert_eq!(
        post(&fixture, &mut store, &fixture.alice, 101, &changed_request),
        Err(ChatStoreError::Conflict)
    );
    before.assert_unchanged(&fixture);
    let changed_message = signed(
        &fixture,
        &fixture.alice,
        81,
        1,
        "different immutable message",
    );
    assert_eq!(
        post(&fixture, &mut store, &fixture.alice, 120, &changed_message),
        Err(ChatStoreError::Conflict)
    );
    before.assert_unchanged(&fixture);
    let reused_position = signed(
        &fixture,
        &fixture.alice,
        82,
        1,
        "same author stream position",
    );
    assert_eq!(
        post(&fixture, &mut store, &fixture.alice, 121, &reused_position),
        Err(ChatStoreError::Conflict)
    );
    before.assert_unchanged(&fixture);
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(fixture.post(&mut store, 101, &original), Ok(expected));
    assert_eq!(fixture.post(&mut store, 102, &original), Ok(expected));
    assert_eq!(
        fixture.permitted_history(&mut store, 122).unwrap().entries,
        vec![HistoryEntry {
            receiver_cursor: 1,
            signed: original
        }]
    );
    before.assert_unchanged(&fixture);
}

#[test]
fn two_author_streams_paginate_locally_without_global_source_sequence() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let alice = fixture.signed_message();
    fixture.post(&mut store, 101, &alice).unwrap();
    let bob = signed(&fixture, &fixture.bob, 82, 1, "Bob source sequence one");
    assert_eq!(
        post(&fixture, &mut store, &fixture.bob, 202, &bob),
        Ok(ChatReceipt {
            message: [82; 16],
            author: author(&fixture.bob),
            source_sequence: 1,
            receiver_cursor: 2,
            original_request: RequestId::from_bytes([202; 16]),
        })
    );
    let known = store.known_frontiers().unwrap();
    assert_eq!(
        known,
        KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 2,
            membership_revision: 1
        }
    );
    let entries = vec![
        HistoryEntry {
            receiver_cursor: 1,
            signed: alice,
        },
        HistoryEntry {
            receiver_cursor: 2,
            signed: bob,
        },
    ];
    assert_eq!(
        history(
            &fixture,
            &mut store,
            &fixture.bob,
            &query(&fixture.bob, 210, 0, 1)
        ),
        Ok(HistoryPage {
            entries: vec![entries[0].clone()],
            next_cursor: 1
        })
    );
    assert_eq!(
        history(
            &fixture,
            &mut store,
            &fixture.bob,
            &query(&fixture.bob, 211, 1, 1)
        ),
        Ok(HistoryPage {
            entries: vec![entries[1].clone()],
            next_cursor: 2
        })
    );
    for (request, after) in [(212, 2), (213, u64::MAX)] {
        assert_eq!(
            history(
                &fixture,
                &mut store,
                &fixture.bob,
                &query(&fixture.bob, request, after, 1)
            ),
            Ok(HistoryPage {
                entries: vec![],
                next_cursor: after
            })
        );
    }
    for (request, limit) in [(214, 2), (215, 64)] {
        assert_eq!(
            history(
                &fixture,
                &mut store,
                &fixture.bob,
                &query(&fixture.bob, request, 0, limit)
            ),
            Ok(HistoryPage {
                entries: entries.clone(),
                next_cursor: 2
            })
        );
    }
    let before = Snapshot::take(&fixture);
    for (request, limit) in [(216, 0), (217, 65)] {
        assert_eq!(
            store.issue_challenge(
                ChallengeRequest::History(&query(&fixture.bob, request, 0, limit)),
                &fixture.bob.public.peer
            ),
            Err(ChatStoreError::Limit)
        );
    }
    before.assert_unchanged(&fixture);
    drop(store);
    let mut store = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(
        history(
            &fixture,
            &mut store,
            &fixture.bob,
            &query(&fixture.bob, 218, 0, 64)
        ),
        Ok(HistoryPage {
            entries,
            next_cursor: 2
        })
    );
}

#[test]
fn canonical_utf8_bounds_and_live_ticket_cap_fail_closed() {
    let fixture = Fixture::new();
    let mut store =
        ChatStore::create(fixture.database(), &fixture.policy, &fixture.membership).unwrap();
    let valid = signed(&fixture, &fixture.alice, 83, 1, &"é".repeat(1024));
    assert_eq!(valid.message.text.len(), 2048);
    let receipt = post(&fixture, &mut store, &fixture.alice, 150, &valid).unwrap();
    assert_eq!(receipt.receiver_cursor, 1);
    let known = store.known_frontiers().unwrap();
    let before = Snapshot::take(&fixture);
    // Invalid canonical inputs cannot be signed by the strict public codec. Retain the
    // prior signature: validation refuses before signature verification or ticket creation.
    let mut oversized = valid.clone();
    oversized.message.text.push('a');
    assert_eq!(oversized.message.text.len(), 2049);
    assert_eq!(
        issue_post(&mut store, &fixture.alice, 151, &oversized),
        Err(ChatStoreError::Limit)
    );
    let mut empty = valid.clone();
    empty.message.text.clear();
    let mut zero_id = valid.clone();
    zero_id.message.message = [0; 16];
    let mut zero_sequence = valid.clone();
    zero_sequence.message.sequence = 0;
    for (request, message) in [(152, empty), (153, zero_id), (154, zero_sequence)] {
        assert_eq!(
            issue_post(&mut store, &fixture.alice, request, &message),
            Err(ChatStoreError::Malformed)
        );
    }
    assert_eq!(
        post(&fixture, &mut store, &fixture.alice, 150, &valid),
        Ok(receipt)
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    before.assert_unchanged(&fixture);
    let query = query(&fixture.bob, 180, 0, 64);
    let started = Instant::now();
    let mut tickets = Vec::new();
    for _ in 0..64 {
        tickets.push(
            store
                .issue_challenge(ChallengeRequest::History(&query), &fixture.bob.public.peer)
                .expect("CHAT_TICKET_CAP_SETUP_ISSUANCE_FAILED"),
        );
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "CHAT_TICKET_CAP_SETUP_EXPIRED_NOT_ACCEPTANCE"
    );
    assert_eq!(
        store.issue_challenge(ChallengeRequest::History(&query), &fixture.bob.public.peer),
        Err(ChatStoreError::Limit)
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "CHAT_TICKET_CAP_SETUP_EXPIRED_NOT_ACCEPTANCE"
    );
    let issued = tickets[0];
    let proof = proof(&fixture.bob, fixture.policy.scope, issued);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "CHAT_TICKET_CAP_SETUP_EXPIRED_NOT_ACCEPTANCE"
    );
    let page = store
        .history(
            &query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &fixture.bob.public.peer,
            },
        )
        .expect("CHAT_TICKET_CAP_CONSUMPTION_EXPIRED_OR_FAILED_NOT_ACCEPTANCE");
    assert_eq!(
        page.entries,
        vec![HistoryEntry {
            receiver_cursor: 1,
            signed: valid
        }]
    );
    assert!(
        store
            .issue_challenge(ChallengeRequest::History(&query), &fixture.bob.public.peer)
            .is_ok()
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    before.assert_unchanged(&fixture);
}

#[test]
fn signed_stored_projection_tamper_refuses_before_configuration() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let signed = fixture.signed_message();
    fixture.post(&mut store, 101, &signed).unwrap();
    fixture.post(&mut store, 102, &signed).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    {
        let connection = rusqlite::Connection::open(&database).unwrap();
        let before: (Vec<u8>, Vec<u8>) = connection
            .query_row(
                "SELECT body,signature FROM chat_messages WHERE message=?1",
                [signed.message.message.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(before.0, codec::message_bytes(&signed.message).unwrap());
        assert_eq!(before.1, signed.signature);
        assert_eq!(
            connection
                .execute(
                    "UPDATE chat_messages SET account=?1 WHERE message=?2",
                    rusqlite::params![
                        fixture.bob.public.account.as_bytes(),
                        signed.message.message.as_slice()
                    ]
                )
                .unwrap(),
            1
        );
        let after: (Vec<u8>, Vec<u8>) = connection
            .query_row(
                "SELECT body,signature FROM chat_messages WHERE message=?1",
                [signed.message.message.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(after, before);
    }
    // Every store/SQL handle is closed before this refused-open byte/name snapshot.
    let tampered = Snapshot::take(&fixture);
    assert!(
        matches!(
            ChatStore::open_existing(&database, &fixture.policy, known),
            Err(ChatStoreError::Corrupt)
        ),
        "CHAT_SIGNED_AUTHOR_CACHE_MISMATCH_MUST_REFUSE"
    );
    tampered.assert_unchanged(&fixture);
}

#[test]
fn independent_receiver_histories_preserve_opposite_arrival_without_global_sender_order() {
    use std::{fs, fs::OpenOptions, io::Write, path::PathBuf};

    fn retain(path: &std::path::Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }

    let started = Instant::now();
    let evidence = PathBuf::from(
        std::env::var_os("NF_CHAT_LOCAL_MUTE_PREFERENCES_EVIDENCE_DIR")
            .expect("Caller must supply one owned external evidence directory"),
    );
    assert!(evidence.is_absolute());
    let evidence = evidence.canonicalize().unwrap();
    assert!(evidence.is_dir());
    let fixture = Fixture::new();
    assert!(!evidence.starts_with(fixture.scratch.0.canonicalize().unwrap()));
    let evidence = evidence.join("receiver-order");
    fs::create_dir(&evidence).unwrap();

    // Both complete originals exist before either independent receiver admits one.
    let alice = fixture.signed_message();
    let bob = signed(
        &fixture,
        &fixture.bob,
        82,
        1,
        "public Bob independent sequence one",
    );
    nf_contract::signatures::verify_digest(
        &fixture.alice.public.device_key,
        &codec::message_digest(&alice.message).unwrap(),
        &alice.signature,
    )
    .unwrap();
    nf_contract::signatures::verify_digest(
        &fixture.bob.public.device_key,
        &codec::message_digest(&bob.message).unwrap(),
        &bob.signature,
    )
    .unwrap();
    assert_ne!(alice.message.author.account, bob.message.author.account);
    assert_eq!(alice.message.sequence, 1);
    assert_eq!(bob.message.sequence, 1);
    retain(
        &evidence.join("alice.signed.original.bin"),
        &codec::encode_signed_message(&alice).unwrap(),
    );
    retain(
        &evidence.join("bob.signed.original.bin"),
        &codec::encode_signed_message(&bob).unwrap(),
    );

    let left_path = fixture.scratch.0.join("receiver-left.sqlite");
    let right_path = fixture.scratch.0.join("receiver-right.sqlite");
    let mut left = ChatStore::create(&left_path, &fixture.policy, &fixture.membership).unwrap();
    let mut right = ChatStore::create(&right_path, &fixture.policy, &fixture.membership).unwrap();
    let left_alice = post(&fixture, &mut left, &fixture.alice, 101, &alice).unwrap();
    let left_bob = post(&fixture, &mut left, &fixture.bob, 202, &bob).unwrap();
    let right_bob = post(&fixture, &mut right, &fixture.bob, 202, &bob).unwrap();
    let right_alice = post(&fixture, &mut right, &fixture.alice, 101, &alice).unwrap();
    assert_eq!(
        (left_alice.receiver_cursor, left_bob.receiver_cursor),
        (1, 2)
    );
    assert_eq!(
        (right_alice.receiver_cursor, right_bob.receiver_cursor),
        (2, 1)
    );
    for receipt in [left_alice, left_bob, right_alice, right_bob] {
        assert_eq!(receipt.source_sequence, 1);
    }
    let left_expected = HistoryPage {
        entries: vec![
            HistoryEntry {
                receiver_cursor: 1,
                signed: alice.clone(),
            },
            HistoryEntry {
                receiver_cursor: 2,
                signed: bob.clone(),
            },
        ],
        next_cursor: 2,
    };
    let right_expected = HistoryPage {
        entries: vec![
            HistoryEntry {
                receiver_cursor: 1,
                signed: bob.clone(),
            },
            HistoryEntry {
                receiver_cursor: 2,
                signed: alice.clone(),
            },
        ],
        next_cursor: 2,
    };
    assert_eq!(
        fixture.permitted_history(&mut left, 210).unwrap(),
        left_expected
    );
    assert_eq!(
        fixture.permitted_history(&mut right, 211).unwrap(),
        right_expected
    );
    let left_known = left.known_frontiers().unwrap();
    let right_known = right.known_frontiers().unwrap();
    assert_eq!(
        left_known,
        KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 2,
            membership_revision: 1
        }
    );
    assert_eq!(right_known, left_known);
    drop(left);
    drop(right);
    let left_before = fs::read(&left_path).unwrap();
    let right_before = fs::read(&right_path).unwrap();
    retain(
        &evidence.join("left.before-reopen.original.sqlite"),
        &left_before,
    );
    retain(
        &evidence.join("right.before-reopen.original.sqlite"),
        &right_before,
    );

    let mut left = ChatStore::open_existing(&left_path, &fixture.policy, left_known).unwrap();
    let mut right = ChatStore::open_existing(&right_path, &fixture.policy, right_known).unwrap();
    // New runtime challenges reauthorize retries; each receiver keeps its own receipt.
    assert_eq!(
        post(&fixture, &mut left, &fixture.alice, 101, &alice),
        Ok(left_alice)
    );
    assert_eq!(
        post(&fixture, &mut left, &fixture.bob, 202, &bob),
        Ok(left_bob)
    );
    assert_eq!(
        post(&fixture, &mut right, &fixture.alice, 101, &alice),
        Ok(right_alice)
    );
    assert_eq!(
        post(&fixture, &mut right, &fixture.bob, 202, &bob),
        Ok(right_bob)
    );
    assert_eq!(
        fixture.permitted_history(&mut left, 212).unwrap(),
        left_expected
    );
    assert_eq!(
        fixture.permitted_history(&mut right, 213).unwrap(),
        right_expected
    );
    assert_eq!(left.known_frontiers().unwrap(), left_known);
    assert_eq!(right.known_frontiers().unwrap(), right_known);
    drop(left);
    drop(right);
    let left_after = fs::read(&left_path).unwrap();
    let right_after = fs::read(&right_path).unwrap();
    retain(
        &evidence.join("left.after-reopen.original.sqlite"),
        &left_after,
    );
    retain(
        &evidence.join("right.after-reopen.original.sqlite"),
        &right_after,
    );
    assert_eq!(left_after, left_before);
    assert_eq!(right_after, right_before);
    assert!(started.elapsed() < Duration::from_secs(20));
}
