use super::{Scratch, fixture, vector};
use nf_store::miniature::*;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
pub fn reject_malformed_history() {
    let corpus = include_str!("../../../../docs/world/vectors/store-v2.tsv");
    let cases: Vec<_> = corpus
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            (columns.first() == Some(&"malformed")).then(|| columns[1])
        })
        .collect();
    assert_eq!(cases.len(), 17);
    for name in cases {
        let scratch = Scratch::new();
        let path = scratch.database();
        let (store, _) = fixture::create(&path);
        let known = store.known_frontiers().unwrap();
        drop(store);
        let bytes = vector("malformed", name);
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            c.execute(
                "UPDATE history_meta SET state=?1,digest=?2 WHERE singleton=1",
                params![bytes, digest]
            )
            .unwrap(),
            1
        );
        let stored: (Vec<u8>, Vec<u8>) = c
            .query_row(
                "SELECT state,digest FROM history_meta WHERE singleton=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored.0, bytes);
        assert_eq!(stored.1, digest);
        drop(c);
        let before = std::fs::read(&path).unwrap();
        let result = MiniatureStore::open_existing(&path, known, AuthConfig::default());
        assert!(
            result.is_err(),
            "malformed public corpus case admitted: {name}"
        );
        if matches!(name, "unknown-store-version" | "wrong-implementation") {
            assert!(
                matches!(
                    result,
                    Err(MiniatureStoreError::Storage(
                        nf_store::StoreError::UnsupportedSchema
                    ))
                ),
                "{name}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
}
