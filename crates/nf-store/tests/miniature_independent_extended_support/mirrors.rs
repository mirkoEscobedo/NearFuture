use super::{Scratch, create};
use nf_store::miniature::*;
use rusqlite::Connection;
pub fn reject_corruption() {
    let cases = [
        "INSERT INTO aggregate_state VALUES (zeroblob(16),zeroblob(8))",
        "INSERT INTO module_state VALUES (zeroblob(16),zeroblob(8),zeroblob(8))",
        "UPDATE module_state SET cooldown=x'0000000000000001'",
        "INSERT INTO world_authority VALUES (1,x'0000000000000001',x'0000000000000001',zeroblob(16),zeroblob(16),zeroblob(8))",
        "INSERT INTO request_outcomes VALUES (zeroblob(16),zeroblob(32),zeroblob(32),zeroblob(16),1,NULL,NULL)",
        "INSERT INTO reservations VALUES (zeroblob(16),zeroblob(16),x'0000000000000064',x'0000000000000028')",
        "INSERT INTO outbox VALUES (zeroblob(16),x'0000000000000001',zeroblob(32),21)",
        "UPDATE membership SET revision=x'0000000000000001'",
        "UPDATE snapshots SET head=zeroblob(32), digest=zeroblob(32)",
        "UPDATE history_meta SET head=x'0100000000000000000000000000000000000000000000000000000000000000'",
    ];
    for sql in cases {
        let scratch = Scratch::new();
        let path = scratch.database();
        let (store, _) = create(&path);
        let known = store.known_frontiers().unwrap();
        drop(store);
        let c = Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        drop(c);
        assert!(
            MiniatureStore::open_existing(&path, known, AuthConfig::default()).is_err(),
            "valid-schema corruption admitted: {sql}"
        );
    }
}
