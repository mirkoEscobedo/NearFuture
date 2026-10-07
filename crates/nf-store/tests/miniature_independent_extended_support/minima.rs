use super::{Scratch, attempt, claim, create};
use nf_contract::identity::*;
use nf_identity::model::MembershipRepository;
use nf_kernel::miniature::settle_miniature;
use nf_store::{StoreError, miniature::*};
pub fn backup_frontiers() {
    let scratch = Scratch::new();
    let live = scratch.database();
    let backup = live.with_file_name("older.sqlite");
    let (mut store, mut signer) = create(&live);
    let earlier = store.backup_to(&backup).unwrap();
    claim(&mut store, &mut signer);
    store.prepare(Vec::new(), Vec::new()).unwrap();
    let lease = store
        .issue_challenge(ChallengeRequest::Activity {
            actor: AccountId::from_bytes([4; 16]),
            device: DeviceId::from_bytes([33; 16]),
        })
        .unwrap();
    store.accept_activity(attempt(&lease, &mut signer)).unwrap();
    let batch = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let commit = store.issue_challenge(ChallengeRequest::Commit).unwrap();
    store.commit(&batch, attempt(&commit, &mut signer)).unwrap();
    let mut membership = store
        .load_membership(earlier.storage.scope)
        .unwrap()
        .unwrap();
    let previous = membership.revision;
    membership.revision += 1;
    store
        .commit_membership(Some(previous), &membership)
        .unwrap();
    let later = store.known_frontiers().unwrap();
    assert_eq!(later.storage.event_sequence, EventSeq(1));
    assert_eq!(later.minimum_authority_term, AuthorityTerm(1));
    assert_eq!(later.storage.membership_revision, Some(1));
    assert!(later.storage.store_revision > earlier.storage.store_revision);
    drop(store);
    let restored = MiniatureStore::open_existing(&backup, earlier, AuthConfig::default()).unwrap();
    assert_eq!(restored.world().metadata().tick, WorldTick(0));
    assert!(restored.authority().is_none());
    drop(restored);
    let before = std::fs::read(&backup).unwrap();
    for case in 0..6 {
        let mut minimum = earlier;
        match case {
            0 => minimum.storage.event_sequence = later.storage.event_sequence,
            1 => minimum.storage.store_revision = later.storage.store_revision,
            2 => minimum.storage.membership_revision = later.storage.membership_revision,
            3 => minimum.minimum_authority_term = later.minimum_authority_term,
            4 => minimum = later,
            5 => minimum.storage.scope.history = HistoryId::from_bytes([99; 16]),
            _ => unreachable!(),
        }
        let result = MiniatureStore::open_existing(&backup, minimum, AuthConfig::default());
        let expected = if case == 5 {
            StoreError::Scope
        } else {
            StoreError::StaleBackup
        };
        assert!(
            matches!(result,Err(MiniatureStoreError::Storage(error)) if error==expected),
            "protected minimum case {case}"
        );
        assert_eq!(std::fs::read(&backup).unwrap(), before);
    }
}
