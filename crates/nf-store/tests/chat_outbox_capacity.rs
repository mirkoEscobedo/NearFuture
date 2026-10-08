mod chat_outbox_controls_support;
use chat_outbox_controls_support::{Fixture, canonical};
use nf_contract::identity::RequestId;
use nf_store::chat::ChatStoreError;
use rusqlite::{Connection, params};

#[test]
fn fully_verified_4096_signed_pending_rows_refuse_new_message_but_retain_original_retry() {
    let fixture = Fixture::new();
    let outbox = fixture.seed();
    drop(outbox);
    // Independently canonical-signed batch after the actual public original enqueue.
    // Public open verifies the complete bounded schema, profile and every signature before cap effects.
    let mut connection = Connection::open(fixture.database()).unwrap();
    let tx = connection.transaction().unwrap();
    for sequence in 2u64..=4096 {
        let mut message = fixture.signed.message.clone();
        message.message = [0; 16];
        message.message[8..].copy_from_slice(&sequence.to_be_bytes());
        message.sequence = sequence;
        message.text = "public capacity fixture hello".to_owned();
        let signed = fixture.sign(message);
        let mut request = [0; 16];
        request[8..].copy_from_slice(&sequence.to_be_bytes());
        assert_eq!(
            tx.execute(
                "INSERT INTO outbox_messages VALUES(?1,?2,?3,?4,?5,?6,?7,1,NULL)",
                params![
                    signed.message.message.as_slice(),
                    signed.message.author.account.as_bytes(),
                    signed.message.author.device.as_bytes(),
                    sequence.to_be_bytes().as_slice(),
                    request.as_slice(),
                    canonical(&signed.message),
                    signed.signature.as_slice()
                ]
            )
            .unwrap(),
            1
        );
    }
    assert_eq!(
        tx.execute(
            "UPDATE outbox_meta SET revision=?1 WHERE singleton=1",
            params![4096u64.to_be_bytes().as_slice()]
        )
        .unwrap(),
        1
    );
    tx.commit().unwrap();
    let rows: i64 = connection
        .query_row("SELECT count(*) FROM outbox_messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 4096);
    drop(connection);
    let mut message = fixture.signed.message.clone();
    message.message = [83; 16];
    message.sequence = 4097;
    let signed = fixture.sign(message);
    let request = RequestId::from_bytes([103; 16]);
    let mut outbox = fixture.reopen(4096).unwrap();
    let after_valid_open = fixture.snapshot();
    assert_eq!(outbox.enqueue(request, &signed), Err(ChatStoreError::Limit));
    assert_eq!(fixture.snapshot(), after_valid_open);
    assert_eq!(
        outbox.enqueue(RequestId::from_bytes([101; 16]), &fixture.signed),
        Ok(fixture.expected_original())
    );
    assert_eq!(fixture.snapshot(), after_valid_open);
    drop(outbox);
    let outbox = fixture.reopen(4096).unwrap();
    assert_eq!(
        outbox.entry([81; 16]),
        Ok(Some(fixture.expected_original()))
    );
    drop(outbox);
    let connection = Connection::open_with_flags(
        fixture.database(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let rows: i64 = connection
        .query_row("SELECT count(*) FROM outbox_messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 4096);
    let receipts: i64 = connection
        .query_row(
            "SELECT count(*) FROM outbox_messages WHERE phase<>1 OR receipt IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(receipts, 0);
}
