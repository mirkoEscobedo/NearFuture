use super::{Scratch, fixture};
use nf_store::{StoreError, miniature::*};
use rusqlite::Connection;
pub fn schema_and_mirrors_fail_closed() {
    let cases = [
        ("CREATE TABLE zzz_extra (v INTEGER)", true),
        (
            "CREATE VIEW zzz_view AS SELECT revision FROM history_meta",
            true,
        ),
        (
            "CREATE TRIGGER zzz_trigger AFTER UPDATE ON history_meta BEGIN SELECT 1; END",
            true,
        ),
        ("CREATE INDEX zzz_index ON history_meta(revision)", true),
        ("UPDATE module_state SET draws=x'0000000000000001'", false),
        (
            "UPDATE aggregate_state SET revision=x'0000000000000001'",
            false,
        ),
        ("DELETE FROM membership", false),
    ];
    for (sql, unsupported) in cases {
        let scratch = Scratch::new();
        let path = scratch.database();
        let (store, _) = fixture::create(&path);
        let known = store.known_frontiers().unwrap();
        drop(store);
        let c = Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        drop(c);
        let before = std::fs::read(&path).unwrap();
        let result = MiniatureStore::open_existing(&path, known, AuthConfig::default());
        assert!(
            result.is_err(),
            "unexpected schema or mirror admitted: {sql}"
        );
        if unsupported {
            assert!(
                matches!(
                    result,
                    Err(MiniatureStoreError::Storage(StoreError::UnsupportedSchema))
                ),
                "{sql}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
}
