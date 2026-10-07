mod notification_effect_support;
use nf_transport::{receipt::ReceiptPhase, receipt_effects::AdmittedReceipt};
use notification_effect_support::{lanes::Lanes, repo::RepoFixture};
#[tokio::test]
async fn actual_persisted_receipt_covers_only_its_query_start_generation() {
    let mut f = RepoFixture::new();
    let mut l = Lanes::new(&mut f).await;
    assert!(l.nc.dirty(), "initial subscription always fresh-queries");
    // A real but uncorrelated direct query cannot cover this notification generation.
    l.rc.request_receipt(&mut f.client_repo).unwrap();
    let (_, uncorrelated) = l.receipt(&mut f).await;
    assert!(matches!(
        l.nc.accept_receipt_completion(uncorrelated, &mut f.client_repo),
        Err(nf_transport::PeerError::Replay)
    ));
    assert!(l.nc.dirty());
    l.nc.request_followup(&mut l.rc, &mut f.client_repo)
        .unwrap();
    let (pending, completion) = l.receipt(&mut f).await;
    assert!(matches!(pending,AdmittedReceipt::Status(s) if s.phase==ReceiptPhase::Pending));
    f.repo
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    l.dirty_notice(&mut f).await;
    // This nonce was issued before the real retained transition/notice. Its fresh,
    // persisted result is valid, but cannot cover the later dirty generation.
    l.nc.accept_receipt_completion(completion, &mut f.client_repo)
        .unwrap();
    assert!(l.nc.dirty());
    l.nc.request_followup(&mut l.rc, &mut f.client_repo)
        .unwrap();
    let (committed, completion) = l.receipt(&mut f).await;
    assert!(
        matches!(committed,AdmittedReceipt::Status(s) if matches!(s.phase,ReceiptPhase::Committed{sequence:nf_contract::identity::EventSeq(1)}))
    );
    l.nc.accept_receipt_completion(completion, &mut f.client_repo)
        .unwrap();
    assert!(!l.nc.dirty());
    l.shutdown();
}
