mod chat_outbox_support;
use chat_outbox_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::chat::outbox::{ClientOutbox, OutgoingEntry, OutgoingState};

#[test]
fn signed_original_is_durable_pending_without_a_receiver_receipt() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut outbox = ClientOutbox::create(&database, &fixture.profile).unwrap();
    let original_request = RequestId::from_bytes([101; 16]);
    let expected = OutgoingEntry {
        original_request,
        signed: fixture.signed.clone(),
        state: OutgoingState::Pending,
    };
    assert_eq!(outbox.known_revision(), Ok(0));
    assert_eq!(outbox.entry([81; 16]), Ok(None));
    // First RED must reach the authenticated enqueue seam: literal Pending versus UnsupportedOperation.
    assert_eq!(
        outbox.enqueue(original_request, &fixture.signed),
        Ok(expected.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
    assert_eq!(
        outbox.enqueue(original_request, &fixture.signed),
        Ok(expected.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    drop(outbox);
    let mut outbox = ClientOutbox::open_existing(&database, &fixture.profile, 1).unwrap();
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
    assert_eq!(
        outbox.enqueue(original_request, &fixture.signed),
        Ok(expected.clone())
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    drop(outbox);
    let outbox = ClientOutbox::open_existing(&database, &fixture.profile, 1).unwrap();
    assert_eq!(outbox.entry([81; 16]), Ok(Some(expected)));
    assert_eq!(outbox.known_revision(), Ok(1));
}
