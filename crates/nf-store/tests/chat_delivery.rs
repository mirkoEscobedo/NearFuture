mod chat_support;
use chat_support::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_store::chat::{ChatReceipt, ChatStore, HistoryEntry, HistoryPage, KnownChatFrontiers};

#[test]
fn a_current_signed_universe_message_is_delivered_once_after_fresh_retry_alias_and_reopen() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    assert_eq!(
        store.known_frontiers().unwrap(),
        KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 0,
            membership_revision: 1,
        }
    );
    assert_eq!(
        fixture.permitted_history(&mut store, 100).unwrap(),
        HistoryPage {
            entries: Vec::new(),
            next_cursor: 0,
        }
    );
    let signed = fixture.signed_message();
    let expected = ChatReceipt {
        message: [81; 16],
        author: author(&fixture.alice),
        source_sequence: 1,
        receiver_cursor: 1,
        original_request: RequestId::from_bytes([101; 16]),
    };
    // The intended RED is authenticated UnsupportedOperation versus this literal receipt.
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(expected));
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(expected));
    assert_eq!(fixture.post(&mut store, 102, &signed), Ok(expected));
    let known = store.known_frontiers().unwrap();
    assert_eq!(
        known,
        KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 1,
            membership_revision: 1,
        }
    );
    assert_eq!(
        fixture.permitted_history(&mut store, 104).unwrap(),
        HistoryPage {
            entries: vec![HistoryEntry {
                receiver_cursor: 1,
                signed: signed.clone()
            }],
            next_cursor: 1,
        }
    );
    drop(store);
    let mut store = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(expected));
    assert_eq!(fixture.post(&mut store, 103, &signed), Ok(expected));
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(
        fixture.permitted_history(&mut store, 105).unwrap(),
        HistoryPage {
            entries: vec![HistoryEntry {
                receiver_cursor: 1,
                signed
            }],
            next_cursor: 1,
        }
    );
}
