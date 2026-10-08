mod chat_revocation_support;
mod chat_support;
use chat_revocation_support::{Snapshot, proof};
use chat_support::{Fixture, author};
use nf_contract::identity::{HistoryId, RequestId};
use nf_identity::{
    model::{DeviceRevocation, IdentityError},
    rotation::revocation_digest,
};
use nf_store::chat::{
    ChallengeRequest, ChatReceipt, ChatStore, ChatStoreError, HistoryEntry, HistoryPage,
    KnownChatFrontiers, ProofAttempt,
};

enum Refusal {
    WrongAccountSignature,
    UnauthorizedOtherAccount,
    OldFrontier,
    OtherHistory,
}
#[test]
fn wrong_account_signature_preserves_current_membership_and_original_history() {
    exercise_refusal(Refusal::WrongAccountSignature, IdentityError::Signature);
}
#[test]
fn genuinely_signed_other_player_cannot_revoke_the_owner_device() {
    exercise_refusal(Refusal::UnauthorizedOtherAccount, IdentityError::RolePolicy);
}
#[test]
fn genuinely_signed_old_revocation_frontier_has_no_effects() {
    exercise_refusal(Refusal::OldFrontier, IdentityError::Frontier);
}
#[test]
fn genuinely_signed_other_history_revocation_has_no_effects() {
    exercise_refusal(Refusal::OtherHistory, IdentityError::Scope);
}
fn exercise_refusal(kind: Refusal, expected: IdentityError) {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let signed = fixture.signed_message();
    let receipt = ChatReceipt {
        message: [81; 16],
        author: author(&fixture.alice),
        source_sequence: 1,
        receiver_cursor: 1,
        original_request: RequestId::from_bytes([101; 16]),
    };
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(receipt));
    let history = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: signed.clone(),
        }],
        next_cursor: 1,
    };
    assert_eq!(
        fixture.permitted_history(&mut store, 100),
        Ok(history.clone())
    );
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    assert_eq!(store.known_frontiers().unwrap(), known);
    let request = RequestId::from_bytes([101; 16]);
    let issued = store
        .issue_challenge(
            ChallengeRequest::Post {
                request,
                message: &signed,
            },
            &fixture.alice.public.peer,
        )
        .unwrap();
    assert_eq!(issued.membership_revision, 1);
    let pending_proof = proof(&fixture.alice, fixture.policy.scope, issued);
    let mut change = DeviceRevocation {
        scope: fixture.policy.scope,
        issuer: fixture.alice.public.account,
        device: fixture.alice.public.device,
        frontier: 1,
    };
    let signer = match kind {
        Refusal::WrongAccountSignature => &fixture.bob.account_key,
        Refusal::UnauthorizedOtherAccount => {
            change.issuer = fixture.bob.public.account;
            &fixture.bob.account_key
        }
        Refusal::OldFrontier => {
            change.frontier = 0;
            &fixture.alice.account_key
        }
        Refusal::OtherHistory => {
            change.scope.history = HistoryId::from_bytes([3; 16]);
            &fixture.alice.account_key
        }
    };
    let signature = signer.sign(&revocation_digest(&change));
    let before = Snapshot::take(&fixture);
    assert_eq!(
        store.revoke_device(&change, &signature),
        Err(ChatStoreError::Identity(expected))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    before.assert_unchanged(&fixture);
    assert_eq!(
        fixture.permitted_history(&mut store, 102),
        Ok(history.clone())
    );
    // A failed account-authority attempt must not advance membership or invalidate a current ticket.
    assert_eq!(
        store.post(
            request,
            &signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &pending_proof,
                peer: &fixture.alice.public.peer,
            }
        ),
        Ok(receipt)
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    before.assert_unchanged(&fixture);
    drop(store);
    assert_durable_membership_and_counts(&fixture);
    let mut store = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    // Successful open may configure SQLite; refusal/read-only bytes are compared only afterward.
    let after_valid_open = Snapshot::take(&fixture);
    assert_eq!(fixture.permitted_history(&mut store, 103), Ok(history));
    after_valid_open.assert_unchanged(&fixture);
    drop(store);
    assert_durable_membership_and_counts(&fixture);
}
fn assert_durable_membership_and_counts(fixture: &Fixture) {
    let connection = rusqlite::Connection::open_with_flags(
        fixture.database(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (revision, body): (Vec<u8>, Vec<u8>) = connection
        .query_row("SELECT revision,public_state FROM membership", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(revision, 1u64.to_be_bytes());
    assert_eq!(
        body,
        nf_identity::codec::encode_state(&fixture.membership).unwrap()
    );
    let actual = nf_identity::codec::decode_state(&body).unwrap();
    // Complete public-state comparison includes both original account/device keys and revocation flags.
    assert_eq!(actual, fixture.membership);
    let requests: i64 = connection
        .query_row("SELECT count(*) FROM chat_requests", [], |row| row.get(0))
        .unwrap();
    let messages: i64 = connection
        .query_row("SELECT count(*) FROM chat_messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!((requests, messages), (1, 1));
    drop(connection);
}
