mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{
    Burn, BurnReason, RejectedOutcome, RequestOutcome, RequestRejection, Reserve, StatusQuery,
    SuppliesRejection, policy_digest,
};
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};

fn reservation(fixture: &Fixture) -> Reserve {
    let own = fixture.query();
    Reserve {
        request: RequestId::from_bytes([44; 16]),
        reservation: OperationId::from_bytes([45; 16]),
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        policy: policy_digest(&fixture.policy).unwrap(),
        content: own.content,
        origin: own.origin,
        amount: 8,
    }
}
#[test]
fn reserved_status_returns_original_and_aliased_request_outcomes_after_checkpoint() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual signed grant25");
    let reserve = reservation(&fixture);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let original = store
        .reserve(&reserve, proof)
        .unwrap()
        .expect("actual signed reservation");
    assert_eq!(
        (original.reservation, original.revision),
        (reserve.reservation, 2)
    );
    let mut alias = reserve;
    alias.request = RequestId::from_bytes([48; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    assert_eq!(store.reserve(&alias, proof).unwrap(), Some(original));
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 2);
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    for request in [reserve.request, alias.request] {
        let query = StatusQuery {
            actor: reserve.actor,
            device: reserve.device,
            owner: reserve.owner,
            universe: reserve.universe,
            history: reserve.history,
            request,
        };
        let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&query));
        assert_eq!(
            store.status(&query, proof).unwrap(),
            Some(RequestOutcome::Reserved(original))
        );
    }
    let own = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    let balance = store.balance(&own, proof).unwrap();
    assert_eq!(
        (
            balance.available,
            balance.reserved,
            balance.minted,
            balance.burned,
            balance.pending,
            balance.externalized
        ),
        (17, 8, 25, 0, 0, 0)
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
}
#[test]
fn reservation_retries_after_later_available_burn_preserve_original_outcome_and_revision() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual signed grant25");
    let reserve = reservation(&fixture);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let original = store
        .reserve(&reserve, proof)
        .unwrap()
        .expect("actual original reserve8");
    assert_eq!(original.revision, 2);
    let burn = Burn {
        request: RequestId::from_bytes([46; 16]),
        burn: OperationId::from_bytes([47; 16]),
        actor: fixture.issuer.account,
        device: fixture.issuer.device,
        owner: reserve.owner,
        universe: reserve.universe,
        history: reserve.history,
        policy: reserve.policy,
        content: reserve.content,
        origin: reserve.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: 17,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Burn(&burn));
    assert_eq!(
        store
            .burn(&burn, proof)
            .unwrap()
            .expect("actual later available burn17")
            .revision,
        3
    );
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 3);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let exact = store.reserve(&reserve, proof);
    let mut alias = reserve;
    alias.request = RequestId::from_bytes([48; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    let fresh_request = store.reserve(&alias, proof);
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let exact_after_checkpoint = store.reserve(&reserve, proof);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    let persisted_alias = store.reserve(&alias, proof);
    let mut later_alias = reserve;
    later_alias.request = RequestId::from_bytes([49; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&later_alias));
    let later_fresh_request = store.reserve(&later_alias, proof);
    let own = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    let balance = store.balance(&own, proof).unwrap();
    assert_eq!(
        (
            balance.available,
            balance.reserved,
            balance.minted,
            balance.burned,
            balance.pending,
            balance.externalized
        ),
        (0, 8, 25, 17, 0, 0)
    );
    for retry in [
        exact,
        fresh_request,
        exact_after_checkpoint,
        persisted_alias,
        later_fresh_request,
    ] {
        assert_eq!(
            retry.expect("dedupe precedes affordability after later burn"),
            Some(original)
        );
    }
    assert_eq!(store.known_frontiers().unwrap(), known);
    let mut attempted_reserved_burn = burn;
    attempted_reserved_burn.request = RequestId::from_bytes([50; 16]);
    attempted_reserved_burn.burn = OperationId::from_bytes([51; 16]);
    attempted_reserved_burn.amount = 1;
    let proof = fixture.attempt(&mut store, ChallengeRequest::Burn(&attempted_reserved_burn));
    assert_eq!(
        store.burn(&attempted_reserved_burn, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    assert_eq!(store.balance(&own, proof).unwrap(), balance);
    assert_eq!(
        store.known_frontiers().unwrap(),
        nf_store::supplies::KnownSuppliesFrontiers {
            revision: 4,
            ..known
        }
    );
    let denied = StatusQuery {
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        request: attempted_reserved_burn.request,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&denied));
    assert_eq!(
        store.status(&denied, proof).unwrap(),
        Some(RequestOutcome::Rejected(RejectedOutcome {
            operation: attempted_reserved_burn.burn,
            revision: 4,
            reason: RequestRejection::InsufficientAvailable,
        }))
    );
}
#[test]
fn unsupported_schema2_and3_markers_are_refused_without_migration_or_file_mutation() {
    for marker in [2, 3] {
        let scratch = Scratch::new();
        let fixture = Fixture::new();
        let mut store =
            SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
        let issuance = fixture.issuance();
        let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
        store
            .issue(&issuance, proof)
            .unwrap()
            .expect("actual signed retained issuance record supported by old2");
        let known = store.known_frontiers().unwrap();
        drop(store);
        // Unsupported selected format markers are refused before schema admission; no old-schema compatibility claim.
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        sql.pragma_update(None, "user_version", marker).unwrap();
        drop(sql);
        let before = std::fs::read(scratch.db()).unwrap();
        let names = || {
            let mut names: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            names.sort();
            names
        };
        let names_before = names();
        let result = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known);
        assert!(
            matches!(result, Err(Error::UnsupportedProfile)),
            "unsupported marker must refuse rather than migrate"
        );
        assert!(
            std::fs::read(scratch.db()).unwrap() == before,
            "unsupported marker refusal preserves every file byte"
        );
        assert!(
            names() == names_before,
            "refusal creates no sidecar or replacement file"
        );
        let sql = rusqlite::Connection::open_with_flags(
            scratch.db(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let version: i32 = sql
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, marker);
    }
}
