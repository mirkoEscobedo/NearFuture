use crate::{Error, Scratch, TradeFixture, TradeRejection, TradeStore};
use nf_kernel::trade::{
    ReserveOffer, economic_reserve_offer_digest, reserve_offer_bytes, reserve_offer_digest,
};
use nf_store::{supplies::SuppliesStoreError, trade::KnownTradeFrontiers};
fn retained() -> (Scratch, TradeFixture, KnownTradeFrontiers, ReserveOffer) {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = f.funded(&scratch);
    let value = f.reserve_offer();
    assert_eq!(
        f.refuse(&mut store, &value),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    f.grant(&mut store, f.taker(), 1, 86, 87, 4);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 4);
    assert_eq!(f.balance(&mut store, f.taker()), [5, 0, 0, 0, 5, 0]);
    drop(store);
    (scratch, f, known, value)
}
fn refused_unchanged(scratch: &Scratch, f: &TradeFixture, known: KnownTradeFrontiers) {
    let names = || {
        let mut entries: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        entries.sort();
        entries
    };
    let before = std::fs::read(scratch.db()).unwrap();
    let entries = names();
    assert!(matches!(
        TradeStore::open_existing(scratch.db(), &f.policy, known),
        Err(Error::Supplies(SuppliesStoreError::Corrupt))
    ));
    assert_eq!(std::fs::read(scratch.db()).unwrap(), before);
    assert_eq!(
        names(),
        entries,
        "rejected replay cannot migrate or materialize sidecars"
    );
}
#[test]
fn replay_refuses_an_insufficient_offer_forged_as_an_accepted_effect() {
    let (scratch, f, known, value) = retained();
    let sql = rusqlite::Connection::open(scratch.db()).unwrap();
    assert_eq!(
        sql.execute(
            "UPDATE supplies_operations SET decision=0 WHERE operation=?1",
            [value.operation.as_bytes()]
        )
        .unwrap(),
        1
    );
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
#[test]
fn replay_rechecks_preceding_stock_even_when_all_forged_body_digests_agree() {
    let (scratch, f, known, mut value) = retained();
    // At refusal revision3 taker had4, so want4 would have been fully funded; later Grant1 is revision4.
    value.terms.want.amount = 4;
    let sql = rusqlite::Connection::open(scratch.db()).unwrap();
    assert_eq!(
        sql.execute(
            "UPDATE supplies_operations SET body=?1,economic_digest=?2 WHERE operation=?3",
            rusqlite::params![
                reserve_offer_bytes(&value),
                economic_reserve_offer_digest(&value),
                value.operation.as_bytes()
            ]
        )
        .unwrap(),
        1
    );
    assert_eq!(
        sql.execute(
            "UPDATE supplies_requests SET body=?1,binding_digest=?2 WHERE request=?3",
            rusqlite::params![
                reserve_offer_bytes(&value),
                reserve_offer_digest(&value),
                value.request.as_bytes()
            ]
        )
        .unwrap(),
        1
    );
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
#[test]
fn replay_requires_the_original_request_of_a_joint_refusal() {
    let (scratch, f, known, value) = retained();
    let sql = rusqlite::Connection::open(scratch.db()).unwrap();
    assert_eq!(
        sql.execute(
            "DELETE FROM supplies_requests WHERE request=?1",
            [value.request.as_bytes()]
        )
        .unwrap(),
        1
    );
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
#[test]
fn replay_refuses_changed_request_binding_and_nonconsecutive_mixed_revision() {
    for binding in [true, false] {
        let (scratch, f, known, value) = retained();
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        let changed = if binding {
            sql.execute(
                "UPDATE supplies_requests SET binding_digest=?1 WHERE request=?2",
                rusqlite::params![[99_u8; 32], value.request.as_bytes()],
            )
        } else {
            sql.execute(
                "UPDATE supplies_operations SET revision=?1 WHERE operation=?2",
                rusqlite::params![7_u64.to_be_bytes(), value.operation.as_bytes()],
            )
        };
        assert_eq!(changed.unwrap(), 1);
        drop(sql);
        refused_unchanged(&scratch, &f, known);
    }
}
