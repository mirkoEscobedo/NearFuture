mod chat_outbox_controls_support;
use chat_outbox_controls_support::Fixture;
use nf_contract::identity::{HistoryId, RequestId};
use nf_store::chat::{
    ChatStoreError, SignedMessage,
    outbox::{OutgoingEntry, OutgoingState},
};

#[test]
fn invalid_author_signature_leaves_original_pending_and_file_unchanged() {
    exercise(ChatStoreError::Signature, |fixture| {
        let mut signed = fixture.signed.clone();
        signed.signature[0] ^= 0x80;
        (RequestId::from_bytes([102; 16]), signed)
    });
}
#[test]
fn genuinely_signed_other_local_author_cannot_enter_the_retained_sender_outbox() {
    let other = Fixture::new();
    exercise(ChatStoreError::Signature, |_| {
        (RequestId::from_bytes([102; 16]), other.signed.clone())
    });
}
#[test]
fn genuinely_signed_other_scope_has_no_pending_effects() {
    exercise(ChatStoreError::Scope, |fixture| {
        let mut message = fixture.signed.message.clone();
        message.scope.history = HistoryId::from_bytes([9; 16]);
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn zero_request_has_no_pending_effects() {
    exercise(ChatStoreError::Malformed, |fixture| {
        (RequestId::from_bytes([0; 16]), fixture.signed.clone())
    });
}
#[test]
fn zero_message_id_has_no_pending_effects() {
    exercise(ChatStoreError::Malformed, |fixture| {
        let mut message = fixture.signed.message.clone();
        message.message = [0; 16];
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn zero_sender_sequence_has_no_pending_effects() {
    exercise(ChatStoreError::Malformed, |fixture| {
        let mut message = fixture.signed.message.clone();
        message.sequence = 0;
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn empty_signed_text_has_no_pending_effects() {
    exercise(ChatStoreError::Malformed, |fixture| {
        let mut message = fixture.signed.message.clone();
        message.text.clear();
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn unicode_text_over_2048_bytes_has_no_pending_effects() {
    exercise(ChatStoreError::Limit, |fixture| {
        let mut message = fixture.signed.message.clone();
        message.text = "é".repeat(1025);
        assert_eq!(message.text.len(), 2050);
        (RequestId::from_bytes([102; 16]), fixture.sign(message))
    });
}
#[test]
fn exactly_2048_utf8_bytes_remain_durable_pending() {
    let fixture = Fixture::new();
    let mut outbox = fixture.seed();
    let mut message = fixture.signed.message.clone();
    message.message = [82; 16];
    message.sequence = 2;
    message.text = "é".repeat(1024);
    assert_eq!(message.text.len(), 2048);
    let signed = fixture.sign(message);
    let request = RequestId::from_bytes([102; 16]);
    let expected = OutgoingEntry {
        original_request: request,
        signed: signed.clone(),
        state: OutgoingState::Pending,
    };
    assert_eq!(outbox.enqueue(request, &signed), Ok(expected.clone()));
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    let outbox = fixture.reopen(2).unwrap();
    assert_eq!(outbox.entry([82; 16]), Ok(Some(expected)));
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    // Snapshot is read only after the actual valid reopened owner.
    let before = fixture.snapshot();
    assert_eq!(outbox.known_revision(), Ok(2));
    assert_eq!(fixture.snapshot(), before);
}
fn exercise(
    expected: ChatStoreError,
    candidate: impl FnOnce(&Fixture) -> (RequestId, SignedMessage),
) {
    let fixture = Fixture::new();
    let (request, signed) = candidate(&fixture);
    let mut outbox = fixture.seed();
    let before = fixture.snapshot();
    assert_eq!(outbox.enqueue(request, &signed), Err(expected));
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    assert_eq!(fixture.snapshot(), before);
    drop(outbox);
    let mut outbox = fixture.reopen(1).unwrap();
    let after_valid_open = fixture.snapshot();
    assert_eq!(outbox.enqueue(request, &signed), Err(expected));
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    assert_eq!(fixture.snapshot(), after_valid_open);
}
