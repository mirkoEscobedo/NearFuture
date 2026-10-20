mod chat_outbox_controls_support;
use chat_outbox_controls_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::chat::{ChatStoreError, SignedMessage};

#[test]
fn same_message_id_cannot_replace_a_real_signed_original() {
    exercise(|fixture| {
        let mut message = fixture.signed.message.clone();
        message.text = "public signed replacement".to_owned();
        (RequestId::from_bytes([101; 16]), fixture.sign(message))
    });
}
#[test]
fn original_request_cannot_alias_a_different_real_signed_message() {
    exercise(|fixture| {
        let mut message = fixture.signed.message.clone();
        message.message = [82; 16];
        message.sequence = 2;
        (RequestId::from_bytes([101; 16]), fixture.sign(message))
    });
}
#[test]
fn same_sender_sequence_cannot_name_another_real_signed_message() {
    exercise(|fixture| {
        let mut message = fixture.signed.message.clone();
        message.message = [82; 16];
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn new_request_cannot_rewrite_an_original_pending_request() {
    exercise(|fixture| (RequestId::from_bytes([102; 16]), fixture.signed.clone()));
}
fn exercise(candidate: impl FnOnce(&Fixture) -> (RequestId, SignedMessage)) {
    let fixture = Fixture::new();
    let (request, signed) = candidate(&fixture);
    let mut outbox = fixture.seed();
    let before = fixture.snapshot();
    assert_eq!(
        outbox.enqueue(request, &signed),
        Err(ChatStoreError::Conflict)
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    assert_eq!(fixture.snapshot(), before);
    drop(outbox);
    let mut outbox = fixture.reopen(1).unwrap();
    let after_valid_open = fixture.snapshot();
    assert_eq!(
        outbox.enqueue(request, &signed),
        Err(ChatStoreError::Conflict)
    );
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(fixture.snapshot(), after_valid_open);
}
