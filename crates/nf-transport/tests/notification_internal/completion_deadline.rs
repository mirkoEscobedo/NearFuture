use super::support::{lanes::Lanes, repo::RepoFixture};
use std::time::Duration;
#[tokio::test]
async fn expired_notification_cannot_cover_dirty_after_validated_receipt_cut() {
    let mut f = RepoFixture::new();
    let mut l = Lanes::with_lifetime(&mut f, 4).await;
    l.nc.request_followup(&mut l.rc, &mut f.client_repo)
        .unwrap();
    let (_, completion) = l.receipt(&mut f).await;
    let remaining = l.nc.remaining_for_test().unwrap();
    assert!(remaining <= Duration::from_secs(4));
    let mut observed = false;
    // This is a simulated synchronous scheduling delay, after real four-lane
    // receipt Status verification/persistence and final current-SQL validation.
    // The private callback receives no clock, authority, keys or Store arguments.
    let result =
        l.nc.accept_completion_after_validation(completion, &mut f.client_repo, || {
            observed = true;
            std::thread::sleep(remaining + Duration::from_millis(50));
        });
    assert!(
        observed,
        "must reach the already-validated real completion cut"
    );
    assert!(
        matches!(result, Err(nf_transport::PeerError::Replay)),
        "expired notification must not cover the dirty generation after resumed scheduling"
    );
    assert!(l.nc.dirty());
    l.shutdown();
}
