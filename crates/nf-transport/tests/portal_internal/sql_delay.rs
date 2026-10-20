//! Real post-SQL work consumes the original public finite lifetime.
use super::{PortalEvent, PortalServer, config, support};
use crate::{PeerError, receipt_effects::ReceiptRepo};
use std::time::{Duration, Instant};
fn fixed_after_current_sql_delay() -> impl Drop {
    ReceiptRepo::delay_current_read_for_test(0)
}
#[tokio::test]
async fn actual_after_sql_delay_exhausts_original_run_before_any_public_lane_output() {
    let f = support::repo::RepoFixture::new();
    let config = config::server_config(&f.repo);
    let mut owner = PortalServer::new(f.repo, config, Duration::from_millis(500)).unwrap();
    let _delay = fixed_after_current_sql_delay();
    let real_elapsed = Instant::now();
    let round = tokio::time::timeout(Duration::from_secs(8), owner.next_round())
        .await
        .unwrap()
        .unwrap();
    assert!(real_elapsed.elapsed() >= Duration::from_millis(5100));
    assert_eq!(round.events.iter().flatten().count(), 1);
    assert!(matches!(round.events[0].as_ref(), Some(PortalEvent::Ended)));
    assert!(matches!(owner.next_round().await, Err(PeerError::Offline)));
}
