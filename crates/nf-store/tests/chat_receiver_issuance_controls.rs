mod chat_receiver_issuance_controls_support;
use chat_receiver_issuance_controls_support::{Fixture, author};
use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::SecretSeed,
    model::{IdentityError, Roles},
};
use nf_store::chat::{ChatStore, ChatStoreError, KnownChatFrontiers, LocalReceiptIssuer};

fn known(fixture: &Fixture, revision: u64, membership_revision: u64) -> KnownChatFrontiers {
    KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision,
        membership_revision,
    }
}
fn refusal(
    fixture: &Fixture,
    mut receiver: ChatStore,
    request: RequestId,
    issuer: LocalReceiptIssuer<'_>,
    error: ChatStoreError,
    frontier: KnownChatFrontiers,
    request_count: i64,
) -> ChatStore {
    // Reconstruct only borrowed selection fields; never clone or expose a private seed.
    let selection = || LocalReceiptIssuer {
        policy: issuer.policy,
        receiver: issuer.receiver,
        peer: issuer.peer,
        device_key: issuer.device_key,
    };
    let before = fixture.snapshot();
    assert_eq!(
        receiver.issue_delivery_receipt(request, selection()),
        Err(error)
    );
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
    drop(receiver);
    // Read-only row inspection is serialized after the exclusive owner is dropped.
    assert_eq!(
        fixture.durable_rows_after_owner_close(),
        (
            frontier.revision as i64,
            request_count,
            frontier.revision.to_be_bytes().to_vec(),
            frontier.membership_revision.to_be_bytes().to_vec(),
        )
    );
    let mut receiver =
        ChatStore::open_existing(fixture.database(), &fixture.policy, frontier).unwrap();
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
    assert_eq!(
        receiver.issue_delivery_receipt(request, selection()),
        Err(error)
    );
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
    receiver
}
#[test]
fn wrong_local_seed_cannot_sign_as_the_retained_receiver() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let wrong = SecretSeed::generate().unwrap();
    let mut issuer = fixture.issuer();
    issuer.device_key = &wrong;
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Signature,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn authentic_deduplicated_alias_never_becomes_the_first_original() {
    let fixture = Fixture::new();
    let mut receiver = fixture.committed_receiver();
    let original = receiver
        .issue_delivery_receipt(RequestId::from_bytes([101; 16]), fixture.issuer())
        .unwrap();
    assert_eq!(
        original.receipt.original.original_request,
        RequestId::from_bytes([101; 16])
    );
    assert_eq!(fixture.verify_signature(&original), Ok(()));
    assert_eq!(
        fixture.post(&mut receiver, RequestId::from_bytes([102; 16])),
        Ok(original.receipt.original)
    );
    assert_eq!(receiver.known_frontiers(), Ok(known(&fixture, 1, 1)));
    let before = fixture.snapshot();
    let mut receiver = refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([102; 16]),
        fixture.issuer(),
        ChatStoreError::Conflict,
        known(&fixture, 1, 1),
        2,
    );
    assert_eq!(
        receiver.issue_delivery_receipt(RequestId::from_bytes([101; 16]), fixture.issuer()),
        Ok(original)
    );
    assert_eq!(fixture.snapshot(), before);
}
#[test]
fn absent_request_is_not_delivery_evidence() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([103; 16]),
        fixture.issuer(),
        ChatStoreError::Conflict,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn zero_original_request_is_malformed() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([0; 16]),
        fixture.issuer(),
        ChatStoreError::Malformed,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn an_uncommitted_signed_message_cannot_yield_a_receipt() {
    let fixture = Fixture::new();
    let receiver = fixture.create_receiver();
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        fixture.issuer(),
        ChatStoreError::Conflict,
        known(&fixture, 0, 1),
        0,
    );
}
#[test]
fn full_peer_identity_must_match_the_retained_receiver() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let mut issuer = fixture.issuer();
    issuer.peer = b"another-authenticated-peer";
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Signature,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn receiver_account_must_own_the_selected_device() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let mut issuer = fixture.issuer();
    issuer.receiver.account = fixture.alice.public.account;
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Signature,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn empty_peer_has_no_local_issuer_authority() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let mut issuer = fixture.issuer();
    issuer.peer = b"";
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Limit,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn oversized_peer_has_no_local_issuer_authority() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let peer = [7; 129];
    let mut issuer = fixture.issuer();
    issuer.peer = &peer;
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Limit,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn an_unenrolled_device_is_not_a_local_receiver() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let (public, _, key) =
        nf_identity::keys::generate_identity(b"unadmitted-control-device".to_vec()).unwrap();
    let issuer = LocalReceiptIssuer {
        policy: fixture.policy,
        receiver: nf_store::chat::Author {
            account: public.account,
            device: public.device,
        },
        peer: &public.peer,
        device_key: &key,
    };
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Identity(IdentityError::UnknownDevice),
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn a_foreign_universe_cannot_request_local_receipt_issuance() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let mut issuer = fixture.issuer();
    issuer.policy.scope.universe = UniverseId::from_bytes([9; 16]);
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Scope,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn a_foreign_history_cannot_request_local_receipt_issuance() {
    let fixture = Fixture::new();
    let receiver = fixture.committed_receiver();
    let mut issuer = fixture.issuer();
    issuer.policy.scope.history = HistoryId::from_bytes([9; 16]);
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        issuer,
        ChatStoreError::Scope,
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn account_signed_receiver_revocation_removes_fresh_issuer_authority() {
    let fixture = Fixture::new();
    let mut receiver = fixture.committed_receiver();
    let next = fixture.revoke(&mut receiver, &fixture.bob).unwrap();
    assert_eq!(next.revision, 2);
    assert!(
        next.devices
            .get(&fixture.bob.public.device)
            .unwrap()
            .revoked
    );
    assert!(
        !next
            .devices
            .get(&fixture.alice.public.device)
            .unwrap()
            .revoked
    );
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        fixture.issuer(),
        ChatStoreError::Identity(IdentityError::Revoked),
        known(&fixture, 1, 2),
        1,
    );
}
#[test]
fn authenticated_relay_membership_is_not_player_receipt_authority() {
    let fixture = Fixture::with_receiver_roles(Roles::RELAY);
    let receiver = fixture.committed_receiver();
    refusal(
        &fixture,
        receiver,
        RequestId::from_bytes([101; 16]),
        fixture.issuer(),
        ChatStoreError::Identity(IdentityError::RolePolicy),
        known(&fixture, 1, 1),
        1,
    );
}
#[test]
fn past_sender_revocation_preserves_the_original_but_refuses_fresh_sender_admission() {
    let fixture = Fixture::new();
    let mut receiver = fixture.committed_receiver();
    let original = receiver
        .issue_delivery_receipt(RequestId::from_bytes([101; 16]), fixture.issuer())
        .unwrap();
    assert_eq!(
        original.receipt.original.original_request,
        RequestId::from_bytes([101; 16])
    );
    assert_eq!(original.receipt.original.author, author(&fixture.alice));
    assert_eq!(fixture.verify_signature(&original), Ok(()));
    let next = fixture.revoke(&mut receiver, &fixture.alice).unwrap();
    assert_eq!(next.revision, 2);
    assert!(
        next.devices
            .get(&fixture.alice.public.device)
            .unwrap()
            .revoked
    );
    assert!(
        !next
            .devices
            .get(&fixture.bob.public.device)
            .unwrap()
            .revoked
    );
    let frontier = known(&fixture, 1, 2);
    let before = fixture.snapshot();
    assert_eq!(
        receiver.issue_delivery_receipt(RequestId::from_bytes([101; 16]), fixture.issuer()),
        Ok(original.clone())
    );
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
    assert_eq!(
        fixture.post(&mut receiver, RequestId::from_bytes([102; 16])),
        Err(ChatStoreError::Identity(IdentityError::Revoked))
    );
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
    drop(receiver);
    assert_eq!(
        fixture.durable_rows_after_owner_close(),
        (
            1,
            1,
            1u64.to_be_bytes().to_vec(),
            2u64.to_be_bytes().to_vec()
        )
    );
    let mut receiver =
        ChatStore::open_existing(fixture.database(), &fixture.policy, frontier).unwrap();
    let retained = receiver
        .issue_delivery_receipt(RequestId::from_bytes([101; 16]), fixture.issuer())
        .unwrap();
    assert_eq!(retained, original);
    assert_eq!(fixture.verify_signature(&retained), Ok(()));
    assert_eq!(receiver.known_frontiers(), Ok(frontier));
    assert_eq!(fixture.snapshot(), before);
}
