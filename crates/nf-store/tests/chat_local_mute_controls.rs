use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_store::chat::{
    Author, Channel, ChatMessage, ChatStoreError, HistoryEntry, HistoryPage, SignedMessage,
    local_view::project_local_mutes,
};

// Synthetic value controls exercise display projection only. The unchanged physical FIRST
// separately proves authenticated history and retained SQL; these values confer no admission.
fn scope() -> Scope {
    Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    }
}
fn account(value: u8) -> AccountId {
    AccountId::from_bytes([value; 16])
}
fn entry(account_value: u8, sequence: u64, cursor: u64, marker: u8) -> HistoryEntry {
    HistoryEntry {
        receiver_cursor: cursor,
        signed: SignedMessage {
            message: ChatMessage {
                scope: scope(),
                channel: Channel::General,
                author: Author {
                    account: account(account_value),
                    device: DeviceId::from_bytes([marker; 16]),
                },
                message: [marker; 16],
                sequence,
                text: format!("display-value-{marker}"),
            },
            signature: [marker; 64],
        },
    }
}

#[test]
fn mixed_authors_keep_signed_values_source_order_and_original_cursor() {
    let page = HistoryPage {
        entries: vec![
            entry(1, 8, 4, 11),
            entry(2, 1, 5, 12),
            entry(1, 2, 6, 13),
            entry(3, 90, 7, 14),
        ],
        next_cursor: 91,
    };
    let original = page.clone();
    let preferences = vec![account(1)];
    let original_preferences = preferences.clone();
    assert_eq!(
        project_local_mutes(scope(), &preferences, &page),
        Ok(HistoryPage {
            entries: vec![original.entries[1].clone(), original.entries[3].clone()],
            next_cursor: 91,
        }),
    );
    assert_eq!(page, original);
    assert_eq!(preferences, original_preferences);
}

#[test]
fn none_all_duplicate_and_empty_views_keep_source_cursor_and_input() {
    let page = HistoryPage {
        entries: vec![entry(1, 8, 4, 11), entry(1, 2, 5, 12)],
        next_cursor: 1000,
    };
    let original = page.clone();
    assert_eq!(
        project_local_mutes(scope(), &[], &page),
        Ok(original.clone())
    );
    for preferences in [vec![account(1)], vec![account(1), account(1)]] {
        let original_preferences = preferences.clone();
        assert_eq!(
            project_local_mutes(scope(), &preferences, &page),
            Ok(HistoryPage {
                entries: Vec::new(),
                next_cursor: 1000
            }),
        );
        assert_eq!(preferences, original_preferences);
    }
    assert_eq!(
        project_local_mutes(scope(), &[], &page),
        Ok(original.clone())
    );
    assert_eq!(page, original);
    let empty = HistoryPage {
        entries: Vec::new(),
        next_cursor: u64::MAX,
    };
    assert_eq!(
        project_local_mutes(scope(), &[account(1)], &empty),
        Ok(empty.clone())
    );
}

#[test]
fn local_snapshot_64_accepts_65_refuses_and_zero_preference_is_malformed() {
    let page = HistoryPage {
        entries: vec![entry(100, 1, 1, 11)],
        next_cursor: 19,
    };
    let original = page.clone();
    let preferences: Vec<_> = (1..=64).map(account).collect();
    let original_preferences = preferences.clone();
    assert_eq!(
        project_local_mutes(scope(), &preferences, &page),
        Ok(original.clone())
    );
    assert_eq!(preferences, original_preferences);
    let mut too_many = preferences;
    too_many.push(account(65));
    let original_too_many = too_many.clone();
    assert_eq!(
        project_local_mutes(scope(), &too_many, &page),
        Err(ChatStoreError::Limit)
    );
    assert_eq!(too_many, original_too_many);
    // Duplicate IDs remain idempotent inside the cap; the snapshot cap counts supplied values.
    assert_eq!(
        project_local_mutes(scope(), &[account(100); 65], &page),
        Err(ChatStoreError::Limit)
    );
    let empty = HistoryPage {
        entries: Vec::new(),
        next_cursor: 19,
    };
    assert_eq!(
        project_local_mutes(scope(), &[account(0)], &empty),
        Err(ChatStoreError::Malformed)
    );
    assert_eq!(page, original);
    assert_eq!(
        empty,
        HistoryPage {
            entries: Vec::new(),
            next_cursor: 19
        }
    );
}

#[test]
fn source_page_64_accepts_and_65_refuses_even_when_all_entries_are_muted() {
    let page = HistoryPage {
        entries: (1..=64)
            .map(|i| entry(1, u64::from(i), u64::from(i), i))
            .collect(),
        next_cursor: 501,
    };
    let original = page.clone();
    assert_eq!(
        project_local_mutes(scope(), &[], &page),
        Ok(original.clone())
    );
    assert_eq!(
        project_local_mutes(scope(), &[account(1)], &page),
        Ok(HistoryPage {
            entries: Vec::new(),
            next_cursor: 501
        }),
    );
    assert_eq!(page, original);
    let mut oversized = page;
    oversized.entries.push(entry(1, 65, 65, 65));
    let original_oversized = oversized.clone();
    assert_eq!(
        project_local_mutes(scope(), &[account(1)], &oversized),
        Err(ChatStoreError::Limit)
    );
    assert_eq!(oversized, original_oversized);
}

#[test]
fn foreign_universe_or_history_refuses_entire_page_before_filtering() {
    for foreign in [
        Scope {
            universe: UniverseId::from_bytes([9; 16]),
            history: scope().history,
        },
        Scope {
            universe: scope().universe,
            history: HistoryId::from_bytes([9; 16]),
        },
    ] {
        let mut alien = entry(2, 1, 5, 12);
        alien.signed.message.scope = foreign;
        let page = HistoryPage {
            entries: vec![entry(1, 8, 4, 11), alien],
            next_cursor: 19,
        };
        let original = page.clone();
        assert_eq!(
            project_local_mutes(scope(), &[account(1), account(2)], &page),
            Err(ChatStoreError::Scope),
        );
        assert_eq!(page, original);
    }
}

#[test]
fn text_2048_byte_boundary_accepts_and_2049_refuses_before_filtering() {
    for text in ["x".repeat(2048), "é".repeat(1024)] {
        assert_eq!(text.len(), 2048);
        let mut bounded = entry(1, 1, 1, 11);
        bounded.signed.message.text = text;
        let page = HistoryPage {
            entries: vec![bounded],
            next_cursor: 19,
        };
        let original = page.clone();
        assert_eq!(
            project_local_mutes(scope(), &[], &page),
            Ok(original.clone())
        );
        assert_eq!(page, original);
        let mut oversized = page;
        oversized.entries[0].signed.message.text.push('x');
        assert_eq!(oversized.entries[0].signed.message.text.len(), 2049);
        let original_oversized = oversized.clone();
        assert_eq!(
            project_local_mutes(scope(), &[account(1)], &oversized),
            Err(ChatStoreError::Limit)
        );
        assert_eq!(oversized, original_oversized);
    }
}
