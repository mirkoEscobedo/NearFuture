mod chat_player_admission_support;
mod chat_revocation_support;
mod chat_support;
use chat_player_admission_support::{history, player, post};
use chat_revocation_support::{Snapshot, proof};
use chat_support::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_identity::{
    model::{Account, Device, IdentityError, Invitation, Roles},
    signing::{admission_proof, sign_invitation},
};
use nf_store::chat::{
    ChallengeRequest, Channel, ChatMessage, ChatReceipt, ChatStore, ChatStoreError, HistoryEntry,
    HistoryPage, KnownChatFrontiers, ProofAttempt, SignedMessage, codec,
};

#[test]
fn locally_supervised_signed_player_admission_retains_history_and_enables_a_new_author_after_reopen()
 {
    let fixture = Fixture::new();
    let charlie = player();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let alice_message = fixture.signed_message();
    assert_eq!(
        fixture.post(&mut store, 101, &alice_message),
        Ok(ChatReceipt {
            message: [81; 16],
            author: author(&fixture.alice),
            source_sequence: 1,
            receiver_cursor: 1,
            original_request: RequestId::from_bytes([101; 16]),
        })
    );
    let original_history = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: alice_message.clone(),
        }],
        next_cursor: 1,
    };
    assert_eq!(
        fixture.permitted_history(&mut store, 100),
        Ok(original_history.clone())
    );
    assert_eq!(
        store.known_frontiers().unwrap(),
        KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 1,
            membership_revision: 1,
        }
    );
    assert_eq!(
        history(&fixture, &charlie, &mut store, 110),
        Err(ChatStoreError::Identity(IdentityError::UnknownDevice))
    );
    let invitation = Invitation {
        scope: fixture.policy.scope,
        id: [7; 16],
        issuer: fixture.alice.public.account,
        recipient: charlie.public.clone(),
        roles: Roles::PLAYER,
        expires_at: 100,
        issued_revision: 1,
        reusable: false,
    };
    let admission =
        admission_proof(&invitation, &charlie.account_key, &charlie.device_key).unwrap();
    let signed = sign_invitation(invitation, &fixture.alice.account_key).unwrap();
    let original_request = RequestId::from_bytes([101; 16]);
    let issued = store
        .issue_challenge(
            ChallengeRequest::Post {
                request: original_request,
                message: &alice_message,
            },
            &fixture.alice.public.peer,
        )
        .unwrap();
    assert_eq!(issued.membership_revision, 1);
    let old_proof = proof(&fixture.alice, fixture.policy.scope, issued);
    // This opaque time comes from the trusted local supervisor, never a sender/wire message.
    let trusted_local_now = 1;
    let admitted = store.admit_player(&signed, &admission, trusted_local_now);
    // Genuine intended RED: all three actual signatures validated, then Unsupported versus member2.
    assert_eq!(admitted.as_ref().map(|state| state.revision), Ok(2));
    let next = admitted.unwrap();
    assert_eq!(next.scope, fixture.policy.scope);
    assert_eq!(next.owner, fixture.membership.owner);
    let mut accounts = fixture.membership.accounts.clone();
    assert!(
        accounts
            .insert(
                charlie.public.account,
                Account {
                    key: charlie.public.account_key,
                    roles: Roles::PLAYER,
                }
            )
            .is_none()
    );
    assert_eq!(next.accounts, accounts);
    let mut devices = fixture.membership.devices.clone();
    assert!(
        devices
            .insert(
                charlie.public.device,
                Device {
                    account: charlie.public.account,
                    key: charlie.public.device_key,
                    peer: charlie.public.peer.clone(),
                    revoked: false,
                }
            )
            .is_none()
    );
    assert_eq!(next.devices, devices);
    let mut consumed = fixture.membership.consumed.clone();
    assert!(consumed.insert([7; 16]));
    assert_eq!(next.consumed, consumed);
    let admitted_frontier = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 2,
    };
    assert_eq!(store.known_frontiers().unwrap(), admitted_frontier);
    let after_admission = Snapshot::take(&fixture);
    assert_eq!(
        store.post(
            original_request,
            &alice_message,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &old_proof,
                peer: &fixture.alice.public.peer,
            }
        ),
        Err(ChatStoreError::Identity(IdentityError::Frontier))
    );
    assert_eq!(
        store.post(
            original_request,
            &alice_message,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &old_proof,
                peer: &fixture.alice.public.peer,
            }
        ),
        Err(ChatStoreError::Replay)
    );
    assert_eq!(
        history(&fixture, &charlie, &mut store, 111),
        Ok(original_history)
    );
    assert_eq!(store.known_frontiers().unwrap(), admitted_frontier);
    after_admission.assert_unchanged(&fixture);
    let message = ChatMessage {
        scope: fixture.policy.scope,
        channel: Channel::General,
        author: author(&charlie),
        message: [82; 16],
        sequence: 1,
        text: "genuine admitted player message".to_owned(),
    };
    let charlie_message = SignedMessage {
        signature: charlie
            .device_key
            .sign(&codec::message_digest(&message).unwrap()),
        message,
    };
    assert_eq!(
        post(&fixture, &charlie, &mut store, 102, &charlie_message),
        Ok(ChatReceipt {
            message: [82; 16],
            author: author(&charlie),
            source_sequence: 1,
            receiver_cursor: 2,
            original_request: RequestId::from_bytes([102; 16]),
        })
    );
    let retained = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 2,
        membership_revision: 2,
    };
    assert_eq!(store.known_frontiers().unwrap(), retained);
    let delivered = HistoryPage {
        entries: vec![
            HistoryEntry {
                receiver_cursor: 1,
                signed: alice_message,
            },
            HistoryEntry {
                receiver_cursor: 2,
                signed: charlie_message,
            },
        ],
        next_cursor: 2,
    };
    let before_replay = Snapshot::take(&fixture);
    assert_eq!(
        store.admit_player(&signed, &admission, trusted_local_now),
        Err(ChatStoreError::Identity(IdentityError::Replay))
    );
    assert_eq!(
        fixture.permitted_history(&mut store, 103),
        Ok(delivered.clone())
    );
    assert_eq!(store.known_frontiers().unwrap(), retained);
    before_replay.assert_unchanged(&fixture);
    drop(store);
    {
        let connection = rusqlite::Connection::open_with_flags(
            &database,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let (revision, body): (Vec<u8>, Vec<u8>) = connection
            .query_row("SELECT revision,public_state FROM membership", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(revision, 2u64.to_be_bytes());
        assert_eq!(nf_identity::codec::decode_state(&body).unwrap(), next);
        let requests: i64 = connection
            .query_row("SELECT count(*) FROM chat_requests", [], |row| row.get(0))
            .unwrap();
        let messages: i64 = connection
            .query_row("SELECT count(*) FROM chat_messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!((requests, messages), (2, 2));
        drop(connection);
    }
    let mut store = ChatStore::open_existing(&database, &fixture.policy, retained).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), retained);
    // Valid reopen may configure SQLite; byte preservation begins only after that successful open.
    let after_valid_open = Snapshot::take(&fixture);
    assert_eq!(
        store.admit_player(&signed, &admission, trusted_local_now),
        Err(ChatStoreError::Identity(IdentityError::Replay))
    );
    assert_eq!(
        history(&fixture, &charlie, &mut store, 112),
        Ok(delivered.clone())
    );
    assert_eq!(fixture.permitted_history(&mut store, 104), Ok(delivered));
    assert_eq!(store.known_frontiers().unwrap(), retained);
    after_valid_open.assert_unchanged(&fixture);
}
