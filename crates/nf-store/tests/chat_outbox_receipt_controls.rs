mod chat_outbox_receipt_controls_support;
use chat_outbox_receipt_controls_support::Fixture;
use nf_contract::identity::{HistoryId, RequestId};
use nf_store::chat::{
    ChatStoreError,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState, SignedChatReceipt},
};
use rusqlite::{Connection, params};

#[test]
fn altered_receiver_signature_cannot_deliver_original_pending() {
    refused(ChatStoreError::Signature, |_, mut candidate| {
        candidate.signature[0] ^= 1;
        candidate
    });
}
#[test]
fn genuine_sender_signature_cannot_choose_the_retained_receiver_key() {
    refused(ChatStoreError::Signature, |fixture, candidate| {
        fixture.sign_with_sender_key(candidate.receipt)
    });
}
#[test]
fn genuine_receiver_signature_for_another_history_cannot_deliver_original() {
    refused(ChatStoreError::Scope, |fixture, mut candidate| {
        candidate.receipt.scope.history = HistoryId::from_bytes([9; 16]);
        fixture.sign_with_receiver_key(candidate.receipt)
    });
}
#[test]
fn genuine_receiver_signature_for_another_policy_cannot_deliver_original() {
    refused(ChatStoreError::Policy, |fixture, mut candidate| {
        candidate.receipt.policy_digest[0] ^= 1;
        fixture.sign_with_receiver_key(candidate.receipt)
    });
}
#[test]
fn genuine_receiver_signature_cannot_replace_the_retained_full_peer_binding() {
    refused(ChatStoreError::Signature, |fixture, mut candidate| {
        candidate.receipt.receiver_peer_digest[0] ^= 1;
        fixture.sign_with_receiver_key(candidate.receipt)
    });
}
#[test]
fn genuine_receiver_signature_cannot_replace_the_original_request() {
    refused(ChatStoreError::Conflict, |fixture, mut candidate| {
        candidate.receipt.original.original_request = RequestId::from_bytes([102; 16]);
        fixture.sign_with_receiver_key(candidate.receipt)
    });
}
#[test]
fn genuine_receiver_signature_cannot_replace_original_signed_message_digest() {
    refused(ChatStoreError::Conflict, |fixture, mut candidate| {
        candidate.receipt.signed_message_digest[0] ^= 1;
        fixture.sign_with_receiver_key(candidate.receipt)
    });
}
#[test]
fn divergent_genuine_receiver_acknowledgment_cannot_rewrite_retained_delivered() {
    let fixture = Fixture::new();
    let mut outbox = seed(&fixture);
    let original = fixture.receiver_commit_reopen_and_issue_receipt();
    let expected = delivered(&fixture, &original);
    assert_eq!(outbox.acknowledge(&original), Ok(expected.clone()));
    let before = fixture.outbox_snapshot();
    let receiver_before = fixture.receiver_snapshot();
    let mut divergent = original.receipt.clone();
    divergent.original.receiver_cursor = 2;
    let divergent = fixture.sign_with_receiver_key(divergent);
    assert_eq!(
        outbox.acknowledge(&divergent),
        Err(ChatStoreError::Conflict)
    );
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
    drop(outbox);
    let outbox =
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 2).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected)));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
}
#[test]
fn truncated_stored_receiver_receipt_is_refused_before_delivered_projection() {
    corrupt(|fixture, original, connection| {
        let mut encoded = fixture.independent_stored_receipt(original);
        encoded.truncate(322);
        update_receipt(connection, &encoded);
    });
}
#[test]
fn trailing_byte_in_stored_receiver_receipt_is_refused_before_projection() {
    corrupt(|fixture, original, connection| {
        let mut encoded = fixture.independent_stored_receipt(original);
        encoded.push(0);
        update_receipt(connection, &encoded);
    });
}
#[test]
fn altered_stored_receipt_domain_is_refused_before_delivered_projection() {
    corrupt(|fixture, original, connection| {
        let mut encoded = fixture.independent_stored_receipt(original);
        encoded[0] ^= 1;
        update_receipt(connection, &encoded);
    });
}
#[test]
fn altered_stored_receiver_signature_is_refused_before_delivered_projection() {
    corrupt(|fixture, original, connection| {
        let mut encoded = fixture.independent_stored_receipt(original);
        encoded[322] ^= 1;
        update_receipt(connection, &encoded);
    });
}
#[test]
fn genuinely_resigned_stored_receipt_for_another_original_digest_is_corrupt() {
    corrupt(|fixture, original, connection| {
        let mut receipt = original.receipt.clone();
        receipt.signed_message_digest[0] ^= 1;
        let candidate = fixture.sign_with_receiver_key(receipt);
        update_receipt(connection, &fixture.independent_stored_receipt(&candidate));
    });
}
#[test]
fn valid_stored_delivered_receipt_requires_the_second_revision() {
    corrupt(|_, _, connection| {
        assert_eq!(
            connection
                .execute(
                    "UPDATE outbox_meta SET revision=?1 WHERE singleton=1",
                    params![1u64.to_be_bytes().as_slice()],
                )
                .unwrap(),
            1,
        );
    });
}
#[test]
fn genuine_older_pending_snapshot_is_refused_against_retained_delivery_revision() {
    let fixture = Fixture::new();
    let mut outbox = seed(&fixture);
    let earlier_pending = fixture.outbox_snapshot();
    let receipt = fixture.receiver_commit_reopen_and_issue_receipt();
    assert_eq!(
        outbox.acknowledge(&receipt),
        Ok(delivered(&fixture, &receipt))
    );
    assert_eq!(outbox.known_revision(), Ok(2));
    let receiver_before = fixture.receiver_snapshot();
    drop(outbox);
    // Restore only this fixture's actual private earlier bytes; retained delivery frontier remains2.
    std::fs::write(fixture.outbox_database(), earlier_pending).unwrap();
    let before = fixture.outbox_snapshot();
    assert_eq!(
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 2).err(),
        Some(ChatStoreError::StaleBackup),
    );
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
    // Old Pending bytes remain valid at their own frontier, with no claim without retained2.
    let outbox =
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 1).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending(&fixture))));
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
}
fn pending(fixture: &Fixture) -> OutgoingEntry {
    OutgoingEntry {
        original_request: RequestId::from_bytes([101; 16]),
        signed: fixture.signed.clone(),
        state: OutgoingState::Pending,
    }
}
fn delivered(fixture: &Fixture, receipt: &SignedChatReceipt) -> OutgoingEntry {
    OutgoingEntry {
        original_request: RequestId::from_bytes([101; 16]),
        signed: fixture.signed.clone(),
        state: OutgoingState::Delivered(Box::new(receipt.clone())),
    }
}
fn seed(fixture: &Fixture) -> ClientOutbox {
    let mut outbox = ClientOutbox::create(fixture.outbox_database(), &fixture.profile).unwrap();
    assert_eq!(
        outbox.enqueue(RequestId::from_bytes([101; 16]), &fixture.signed),
        Ok(pending(fixture)),
    );
    outbox
}
fn refused(
    expected: ChatStoreError,
    change: impl FnOnce(&Fixture, SignedChatReceipt) -> SignedChatReceipt,
) {
    let fixture = Fixture::new();
    let mut outbox = seed(&fixture);
    // Every candidate starts from an actual committed/reopened receiver history and genuine Bob receipt.
    let genuine = fixture.receiver_commit_reopen_and_issue_receipt();
    let candidate = change(&fixture, genuine);
    let before = fixture.outbox_snapshot();
    let receiver_before = fixture.receiver_snapshot();
    assert_eq!(outbox.acknowledge(&candidate), Err(expected));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending(&fixture))));
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
    drop(outbox);
    let outbox =
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 1).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending(&fixture))));
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
}
fn update_receipt(connection: &Connection, encoded: &[u8]) {
    assert_eq!(
        connection
            .execute(
                "UPDATE outbox_messages SET receipt=?1 WHERE message=?2",
                params![encoded, [81u8; 16].as_slice()],
            )
            .unwrap(),
        1,
    );
}
fn corrupt(change: impl FnOnce(&Fixture, &SignedChatReceipt, &Connection)) {
    let fixture = Fixture::new();
    let mut outbox = seed(&fixture);
    let receipt = fixture.receiver_commit_reopen_and_issue_receipt();
    let expected = delivered(&fixture, &receipt);
    assert_eq!(outbox.acknowledge(&receipt), Ok(expected.clone()));
    assert_eq!(fixture.independent_stored_receipt(&receipt).len(), 323);
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected)));
    assert_eq!(outbox.known_revision(), Ok(2));
    let receiver_before = fixture.receiver_snapshot();
    // Close the exclusive owner before the fixture-only SQL mutation.
    drop(outbox);
    let connection = Connection::open(fixture.outbox_database()).unwrap();
    change(&fixture, &receipt, &connection);
    drop(connection);
    let before = fixture.outbox_snapshot();
    assert_eq!(
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 2).err(),
        Some(ChatStoreError::Corrupt),
    );
    assert_eq!(fixture.outbox_snapshot(), before);
    assert_eq!(fixture.receiver_snapshot(), receiver_before);
}
