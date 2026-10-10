mod chat_receiver_issuance_support;
use chat_receiver_issuance_support::{Fixture, canonical_receipt_frame};
use nf_contract::identity::RequestId;
use nf_store::chat::outbox::{ClientOutbox, OutgoingEntry, OutgoingState};
use nf_store::chat::{ChatReceipt, ChatStore, KnownChatFrontiers};

#[test]
fn production_receiver_issues_only_the_durable_first_original_without_advancing_history() {
    let fixture = Fixture::new();
    let request = RequestId::from_bytes([101; 16]);
    let pending = OutgoingEntry {
        original_request: request,
        signed: fixture.signed.clone(),
        state: OutgoingState::Pending,
    };
    let mut outbox = ClientOutbox::create(fixture.outbox_database(), &fixture.profile).unwrap();
    assert_eq!(
        outbox.enqueue(request, &fixture.signed),
        Ok(pending.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    let mut receiver = fixture.receiver_commit_reopen();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    assert_eq!(receiver.known_frontiers(), Ok(known));
    let receiver_before_issuance = fixture.receiver_snapshot();
    let expected = fixture.expected_receipt(ChatReceipt {
        message: [81; 16],
        author: fixture.signed.message.author,
        source_sequence: 1,
        receiver_cursor: 1,
        original_request: request,
    });
    let issued = receiver.issue_delivery_receipt(request, fixture.issuer());
    // First public RED is authenticated UnsupportedOperation versus this independent exact body.
    assert_eq!(
        issued.as_ref().map(|signed| signed.receipt.clone()),
        Ok(expected)
    );
    let receipt = issued.unwrap();
    assert_eq!(canonical_receipt_frame(&receipt.receipt).len(), 259);
    assert_eq!(fixture.verify_signature(&receipt), Ok(()));
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(fixture.receiver_snapshot(), receiver_before_issuance);
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending)));
    let delivered = OutgoingEntry {
        original_request: request,
        signed: fixture.signed.clone(),
        state: OutgoingState::Delivered(Box::new(receipt.clone())),
    };
    assert_eq!(outbox.acknowledge(&receipt), Ok(delivered.clone()));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered.clone())));
    let outbox_after_delivery = fixture.outbox_snapshot();
    assert_eq!(
        receiver.issue_delivery_receipt(request, fixture.issuer()),
        Ok(receipt.clone())
    );
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(fixture.receiver_snapshot(), receiver_before_issuance);
    assert_eq!(outbox.acknowledge(&receipt), Ok(delivered.clone()));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), outbox_after_delivery);
    drop(receiver);
    drop(outbox);
    let mut receiver =
        ChatStore::open_existing(fixture.receiver_database(), &fixture.policy, known).unwrap();
    let outbox =
        ClientOutbox::open_existing(fixture.outbox_database(), &fixture.profile, 2).unwrap();
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered)));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), outbox_after_delivery);
    assert_eq!(
        receiver.issue_delivery_receipt(request, fixture.issuer()),
        Ok(receipt)
    );
    assert_eq!(receiver.known_frontiers(), Ok(known));
    assert_eq!(fixture.receiver_snapshot(), receiver_before_issuance);
}
