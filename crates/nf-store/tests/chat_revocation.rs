mod chat_revocation_support;
mod chat_support;
use chat_revocation_support::{Snapshot, proof};
use chat_support::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_identity::{
    model::{DeviceRevocation, IdentityError},
    rotation::revocation_digest,
};
use nf_store::chat::{
    ChallengeRequest, ChatReceipt, ChatStore, ChatStoreError, HistoryEntry, HistoryPage,
    KnownChatFrontiers, ProofAttempt,
};

#[test]
fn account_signed_live_self_revocation_preserves_delivered_history_and_member_frontier_after_reopen()
 {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let signed = fixture.signed_message();
    assert_eq!(
        fixture.post(&mut store, 101, &signed),
        Ok(ChatReceipt {
            message: [81; 16],
            author: author(&fixture.alice),
            source_sequence: 1,
            receiver_cursor: 1,
            original_request: RequestId::from_bytes([101; 16]),
        })
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
        fixture.permitted_history(&mut store, 100).unwrap().entries,
        vec![HistoryEntry {
            receiver_cursor: 1,
            signed: signed.clone()
        }]
    );
    let alias = RequestId::from_bytes([102; 16]);
    let issued = store
        .issue_challenge(
            ChallengeRequest::Post {
                request: alias,
                message: &signed,
            },
            &fixture.alice.public.peer,
        )
        .unwrap();
    assert_eq!(issued.membership_revision, 1);
    let old_proof = proof(&fixture.alice, fixture.policy.scope, issued);
    let change = DeviceRevocation {
        scope: fixture.policy.scope,
        issuer: fixture.alice.public.account,
        device: fixture.alice.public.device,
        frontier: 1,
    };
    let signature = fixture.alice.account_key.sign(&revocation_digest(&change));
    let result = store.revoke_device(&change, &signature);
    // Genuine intended RED: authenticated UnsupportedOperation versus literal member revision2.
    assert_eq!(result.as_ref().map(|state| state.revision), Ok(2));
    let next = result.unwrap();
    assert_eq!(next.scope, fixture.policy.scope);
    assert_eq!(next.owner, fixture.membership.owner);
    assert_eq!(next.accounts, fixture.membership.accounts);
    assert_eq!(next.consumed, fixture.membership.consumed);
    let mut expected_devices = fixture.membership.devices.clone();
    expected_devices
        .get_mut(&fixture.alice.public.device)
        .unwrap()
        .revoked = true;
    assert_eq!(next.devices, expected_devices);
    let retained = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 2,
    };
    assert_eq!(store.known_frontiers().unwrap(), retained);
    let before_refusals = Snapshot::take(&fixture);
    assert_eq!(
        store.post(
            alias,
            &signed,
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
            alias,
            &signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &old_proof,
                peer: &fixture.alice.public.peer,
            }
        ),
        Err(ChatStoreError::Replay)
    );
    assert_eq!(
        store.issue_challenge(
            ChallengeRequest::Post {
                request: alias,
                message: &signed,
            },
            &fixture.alice.public.peer
        ),
        Err(ChatStoreError::Identity(IdentityError::Revoked))
    );
    assert_eq!(store.known_frontiers().unwrap(), retained);
    before_refusals.assert_unchanged(&fixture);
    assert_eq!(
        fixture.permitted_history(&mut store, 104),
        Ok(HistoryPage {
            entries: vec![HistoryEntry {
                receiver_cursor: 1,
                signed: signed.clone()
            }],
            next_cursor: 1,
        })
    );
    before_refusals.assert_unchanged(&fixture);
    drop(store);
    // Independent read-only durable row observations use a separate connection only after owner close.
    {
        let connection = rusqlite::Connection::open(&database).unwrap();
        let alias_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM chat_requests WHERE request=?1",
                [alias.as_bytes()],
                |row| row.get(0),
            )
            .unwrap();
        let request_count: i64 = connection
            .query_row("SELECT count(*) FROM chat_requests", [], |row| row.get(0))
            .unwrap();
        let message_count: i64 = connection
            .query_row("SELECT count(*) FROM chat_messages", [], |row| row.get(0))
            .unwrap();
        let membership_revision: Vec<u8> = connection
            .query_row("SELECT revision FROM membership", [], |row| row.get(0))
            .unwrap();
        assert_eq!(alias_count, 0);
        assert_eq!(request_count, 1);
        assert_eq!(message_count, 1);
        assert_eq!(membership_revision, 2u64.to_be_bytes());
    }
    let mut store = ChatStore::open_existing(&database, &fixture.policy, retained).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), retained);
    assert_eq!(
        fixture.permitted_history(&mut store, 105),
        Ok(HistoryPage {
            entries: vec![HistoryEntry {
                receiver_cursor: 1,
                signed
            }],
            next_cursor: 1,
        })
    );
}
