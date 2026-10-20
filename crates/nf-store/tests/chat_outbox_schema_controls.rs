mod chat_outbox_controls_support;
use chat_outbox_controls_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::chat::{
    ChatStoreError,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState},
};
use rusqlite::{Connection, params};

#[test]
fn another_genuine_retained_sender_receiver_profile_cannot_open_original_pending() {
    let fixture = Fixture::new();
    let other = Fixture::new();
    let outbox = fixture.seed();
    drop(outbox);
    let before = fixture.snapshot();
    assert_eq!(
        ClientOutbox::open_existing(fixture.database(), &other.profile, 1).err(),
        Some(ChatStoreError::Policy)
    );
    assert_eq!(fixture.snapshot(), before);
    let outbox = fixture.reopen(1).unwrap();
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
}
#[test]
fn corrupted_stored_original_signature_is_refused_before_projection() {
    corrupt(ChatStoreError::Corrupt, |connection| {
        assert_eq!(
            connection
                .execute(
                    "UPDATE outbox_messages SET signature=?1 WHERE message=?2",
                    params![[0u8; 64].as_slice(), [81u8; 16].as_slice()]
                )
                .unwrap(),
            1
        );
    });
}
#[test]
fn reserved_delivered_row_without_verified_receipt_is_refused_before_projection() {
    corrupt(ChatStoreError::Corrupt, |connection| {
        assert_eq!(
            connection
                .execute(
                    "UPDATE outbox_messages SET phase=2,receipt=?1 WHERE message=?2",
                    params![[1u8; 420].as_slice(), [81u8; 16].as_slice()]
                )
                .unwrap(),
            1
        );
    });
}
#[test]
fn extra_schema_table_is_refused_before_pending_projection() {
    corrupt(ChatStoreError::UnsupportedProfile, |connection| {
        connection
            .execute_batch("CREATE TABLE alien(value BLOB) STRICT;")
            .unwrap();
    });
}
#[test]
fn valid_older_pending_file_is_refused_against_retained_newer_revision() {
    let fixture = Fixture::new();
    let mut outbox = fixture.seed();
    let old = fixture.snapshot();
    let mut message = fixture.signed.message.clone();
    message.message = [82; 16];
    message.sequence = 2;
    let signed = fixture.sign(message);
    let request = RequestId::from_bytes([102; 16]);
    let expected = OutgoingEntry {
        original_request: request,
        signed: signed.clone(),
        state: OutgoingState::Pending,
    };
    assert_eq!(outbox.enqueue(request, &signed), Ok(expected));
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    // Replace only this fixture's new private scratch file with its actual earlier valid snapshot.
    std::fs::write(fixture.database(), old).unwrap();
    let before = fixture.snapshot();
    assert_eq!(fixture.reopen(2).err(), Some(ChatStoreError::StaleBackup));
    assert_eq!(fixture.snapshot(), before);
    // Older bytes remain valid at their own frontier; no claim of rollback detection without retained2.
    let outbox = fixture.reopen(1).unwrap();
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
}
fn corrupt(expected: ChatStoreError, change: impl FnOnce(&Connection)) {
    let fixture = Fixture::new();
    let outbox = fixture.seed();
    drop(outbox);
    let connection = Connection::open(fixture.database()).unwrap();
    change(&connection);
    drop(connection);
    let before = fixture.snapshot();
    assert_eq!(fixture.reopen(1).err(), Some(expected));
    assert_eq!(fixture.snapshot(), before);
}
