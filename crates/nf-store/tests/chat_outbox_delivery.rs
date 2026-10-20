mod chat_outbox_delivery_support;
use chat_outbox_delivery_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::chat::outbox::{ClientOutbox, OutgoingEntry, OutgoingState};

#[test]
fn genuine_receiver_signed_receipt_durably_delivers_the_same_original_once() {
    let fixture = Fixture::new();
    let database = fixture.outbox_database();
    let request = RequestId::from_bytes([101; 16]);
    let pending = OutgoingEntry {
        original_request: request,
        signed: fixture.signed.clone(),
        state: OutgoingState::Pending,
    };
    let mut outbox = ClientOutbox::create(&database, &fixture.profile).unwrap();
    assert_eq!(outbox.known_revision(), Ok(0));
    assert_eq!(outbox.entry([81; 16]), Ok(None));
    assert_eq!(
        outbox.enqueue(request, &fixture.signed),
        Ok(pending.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending.clone())));
    // Separate receiver history really commits, closes/reopens and authenticates its local reader.
    // Its local service then issues a genuine retained Bob device signature over an independent frame.
    let receipt = fixture.receiver_commit_reopen_and_issue_receipt();
    let receiver_after_commit = fixture.receiver_snapshot();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(pending)));
    assert_eq!(outbox.known_revision(), Ok(1));
    let delivered = OutgoingEntry {
        original_request: request,
        signed: fixture.signed.clone(),
        state: OutgoingState::Delivered(Box::new(receipt.clone())),
    };
    // First genuine RED: compiling acknowledge validates every pin/signature/original binding
    // before returning UnsupportedOperation without effects, versus this literal Delivered.
    assert_eq!(outbox.acknowledge(&receipt), Ok(delivered.clone()));
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered.clone())));
    let after_delivered = fixture.outbox_snapshot();
    assert_eq!(outbox.acknowledge(&receipt), Ok(delivered.clone()));
    assert_eq!(
        outbox.enqueue(request, &fixture.signed),
        Ok(delivered.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), after_delivered);
    assert_eq!(fixture.receiver_snapshot(), receiver_after_commit);
    drop(outbox);
    let mut outbox = ClientOutbox::open_existing(&database, &fixture.profile, 2).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered.clone())));
    assert_eq!(outbox.known_revision(), Ok(2));
    let after_valid_open = fixture.outbox_snapshot();
    assert_eq!(outbox.acknowledge(&receipt), Ok(delivered.clone()));
    assert_eq!(
        outbox.enqueue(request, &fixture.signed),
        Ok(delivered.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.outbox_snapshot(), after_valid_open);
    assert_eq!(fixture.receiver_snapshot(), receiver_after_commit);
    drop(outbox);
    let outbox = ClientOutbox::open_existing(&database, &fixture.profile, 2).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered)));
    assert_eq!(outbox.known_revision(), Ok(2));
}
