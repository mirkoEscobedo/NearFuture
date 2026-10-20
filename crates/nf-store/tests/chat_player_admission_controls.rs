mod chat_revocation_support;
mod chat_support;
use chat_revocation_support::{Snapshot, proof};
use chat_support::{Fixture, author};
use nf_contract::identity::{HistoryId, RequestId};
use nf_identity::{
    keys::generate_identity,
    model::{IdentityError, Invitation, Roles},
    private_storage::LocalIdentity,
    signing::{admission_proof, sign_invitation},
};
use nf_store::chat::{
    ChallengeRequest, Channel, ChatReceipt, ChatStore, ChatStoreError, HistoryEntry, HistoryPage,
    HistoryQuery, KnownChatFrontiers, ProofAttempt,
};

enum Refusal {
    IssuerSignature,
    RecipientAccountSignature,
    RecipientDeviceSignature,
    ExpiredAtTrustedLocalNow,
    OtherHistory,
    NonPlayerRoles,
    IssuedAheadOfCurrentMembership,
}
#[test]
fn wrong_issuer_signature_cannot_admit_a_real_new_player() {
    exercise_refusal(Refusal::IssuerSignature, IdentityError::Signature);
}
#[test]
fn wrong_recipient_account_signature_cannot_admit_a_real_new_player() {
    exercise_refusal(Refusal::RecipientAccountSignature, IdentityError::Signature);
}
#[test]
fn wrong_recipient_device_signature_cannot_admit_a_real_new_player() {
    exercise_refusal(Refusal::RecipientDeviceSignature, IdentityError::Signature);
}
#[test]
fn invitation_expired_at_trusted_local_now_has_no_membership_or_history_effects() {
    exercise_refusal(Refusal::ExpiredAtTrustedLocalNow, IdentityError::Expired);
}
#[test]
fn genuinely_signed_other_history_invitation_has_no_effects() {
    exercise_refusal(Refusal::OtherHistory, IdentityError::Scope);
}
#[test]
fn genuinely_signed_non_player_invitation_is_refused_by_chat_policy() {
    exercise_refusal(Refusal::NonPlayerRoles, IdentityError::RolePolicy);
}
#[test]
fn genuinely_signed_invitation_issued_ahead_of_current_membership_has_no_effects() {
    exercise_refusal(
        Refusal::IssuedAheadOfCurrentMembership,
        IdentityError::Frontier,
    );
}
fn exercise_refusal(kind: Refusal, expected: IdentityError) {
    let fixture = Fixture::new();
    let (public, account_key, device_key) =
        generate_identity(b"chat-admission-refusal-charlie".to_vec()).unwrap();
    let charlie = LocalIdentity {
        public,
        account_key,
        device_key,
    };
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let message = fixture.signed_message();
    let receipt = ChatReceipt {
        message: [81; 16],
        author: author(&fixture.alice),
        source_sequence: 1,
        receiver_cursor: 1,
        original_request: RequestId::from_bytes([101; 16]),
    };
    assert_eq!(fixture.post(&mut store, 101, &message), Ok(receipt));
    let original_history = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: message.clone(),
        }],
        next_cursor: 1,
    };
    assert_eq!(
        fixture.permitted_history(&mut store, 100),
        Ok(original_history.clone())
    );
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_unknown_player(&charlie, &mut store);
    let mut invitation = Invitation {
        scope: fixture.policy.scope,
        id: [7; 16],
        issuer: fixture.alice.public.account,
        recipient: charlie.public.clone(),
        roles: Roles::PLAYER,
        expires_at: 100,
        issued_revision: 1,
        reusable: false,
    };
    // Opaque local supervisor time: no wall-clock unit or remote timestamp policy is implied.
    let mut trusted_local_now = 1;
    match kind {
        Refusal::ExpiredAtTrustedLocalNow => trusted_local_now = 100,
        Refusal::OtherHistory => invitation.scope.history = HistoryId::from_bytes([3; 16]),
        Refusal::NonPlayerRoles => invitation.roles = Roles::WORKER,
        Refusal::IssuedAheadOfCurrentMembership => invitation.issued_revision = 2,
        _ => {}
    }
    let issuer_key = match kind {
        Refusal::IssuerSignature => &fixture.bob.account_key,
        _ => &fixture.alice.account_key,
    };
    let recipient_account_key = match kind {
        Refusal::RecipientAccountSignature => &fixture.bob.account_key,
        _ => &charlie.account_key,
    };
    let recipient_device_key = match kind {
        Refusal::RecipientDeviceSignature => &fixture.bob.device_key,
        _ => &charlie.device_key,
    };
    // Every candidate uses real keys; only the selected signature signer or signed field differs.
    let admission =
        admission_proof(&invitation, recipient_account_key, recipient_device_key).unwrap();
    let signed = sign_invitation(invitation, issuer_key).unwrap();
    // All identity generation and candidate signing occur BEFORE the five-second held ticket.
    let request = RequestId::from_bytes([101; 16]);
    let issued = store
        .issue_challenge(
            ChallengeRequest::Post {
                request,
                message: &message,
            },
            &fixture.alice.public.peer,
        )
        .unwrap();
    assert_eq!(issued.membership_revision, 1);
    let pending_proof = proof(&fixture.alice, fixture.policy.scope, issued);
    let before = Snapshot::take(&fixture);
    assert_eq!(
        store.admit_player(&signed, &admission, trusted_local_now),
        Err(ChatStoreError::Identity(expected))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_unknown_player(&charlie, &mut store);
    assert_eq!(
        fixture.permitted_history(&mut store, 102),
        Ok(original_history.clone())
    );
    before.assert_unchanged(&fixture);
    // Failed admission must neither change membership nor consume an unrelated current ticket.
    // Expiry is a setup failure and fails this test; it is never accepted as the refusal oracle.
    assert_eq!(
        store.post(
            request,
            &message,
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
    assert_durable_original_membership_and_counts(&fixture);
    let mut store = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    // Successful configure may write SQLite headers; refusal bytes start AFTER actual valid open.
    let after_valid_open = Snapshot::take(&fixture);
    assert_eq!(
        store.admit_player(&signed, &admission, trusted_local_now),
        Err(ChatStoreError::Identity(expected))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_unknown_player(&charlie, &mut store);
    assert_eq!(
        fixture.permitted_history(&mut store, 103),
        Ok(original_history)
    );
    after_valid_open.assert_unchanged(&fixture);
    drop(store);
    assert_durable_original_membership_and_counts(&fixture);
}
fn assert_unknown_player(charlie: &LocalIdentity, store: &mut ChatStore) {
    let query = HistoryQuery {
        request: RequestId::from_bytes([110; 16]),
        reader: author(charlie),
        channel: Channel::General,
        after_cursor: 0,
        limit: 64,
    };
    assert_eq!(
        store.issue_challenge(ChallengeRequest::History(&query), &charlie.public.peer),
        Err(ChatStoreError::Identity(IdentityError::UnknownDevice))
    );
}
fn assert_durable_original_membership_and_counts(fixture: &Fixture) {
    // Independent connection observes only AFTER exclusive ChatStore owner has been dropped.
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
    // Full canonical comparison retains both original account/device keys, roles, flags and consumed IDs.
    assert_eq!(
        nf_identity::codec::decode_state(&body).unwrap(),
        fixture.membership
    );
    let requests: i64 = connection
        .query_row("SELECT count(*) FROM chat_requests", [], |row| row.get(0))
        .unwrap();
    let messages: i64 = connection
        .query_row("SELECT count(*) FROM chat_messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!((requests, messages), (1, 1));
    drop(connection);
}
