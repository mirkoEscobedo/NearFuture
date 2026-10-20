//! Authentic public two-lane startup must lead to a correlated admitted retained status.
use super::{PortalEvent, PortalServer, config, support};
use crate::{
    portal::{PortalClient, WatchEvent, WatchRound},
    portal_config::{OriginalConfig, PortalConfig, PortalMode},
    receipt::{ReceiptPhase, SourceMinima},
    receipt_effects::{AdmittedReceipt, ReceiptRepo},
};
use libp2p::Multiaddr;
use nf_contract::identity::EventSeq;
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

fn watch_config(repo: &ReceiptRepo, addresses: Option<[Multiaddr; 3]>) -> PortalConfig {
    let c = repo.config();
    let catalog = repo.recover_observations().unwrap();
    let head = catalog.head(0).unwrap();
    PortalConfig {
        mode: PortalMode::Watch,
        local: nf_store::KnownFrontiers {
            scope: c.scope,
            event_sequence: EventSeq(0),
            store_revision: 0,
            membership_revision: Some(1),
        },
        ruleset: c.ruleset,
        content: c.content,
        local_account: c.local_account,
        local_device: c.local_device,
        server: c.server_pin,
        addresses,
        originals: vec![OriginalConfig {
            slot: 0,
            original: head.original.clone(),
            minimum: head.minimum,
            head: Some((head.generation, head.digest().unwrap())),
        }],
    }
}
// Passive test-only intervals: scheduler preemption is included, not attributed to I/O.
#[derive(Clone, Copy, Debug, Default)]
struct PollInterval {
    count: u64,
    total: Duration,
    maximum: Duration,
}
impl PollInterval {
    fn record(&mut self, elapsed: Duration) {
        self.count = self.count.saturating_add(1);
        self.total = self.total.saturating_add(elapsed);
        self.maximum = self.maximum.max(elapsed);
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct PollIntervals {
    client: PollInterval,
    server: PollInterval,
    between_polls: PollInterval,
}
#[derive(Clone, Copy, Default)]
struct Measurement {
    intervals: PollIntervals,
    last_poll_end: Option<Instant>,
}
#[derive(Clone, Copy)]
enum PollOwner {
    Client,
    Server,
}
#[derive(Clone, Copy, Debug)]
enum WatchPhase {
    Startup,
    RetainedStatus,
}
fn measure_poll<F: Future>(
    future: Pin<&mut F>,
    cx: &mut Context<'_>,
    owner: PollOwner,
    measurement: &Cell<Measurement>,
) -> Poll<F::Output> {
    let started = Instant::now();
    let mut measured = measurement.get();
    if let Some(previous) = measured.last_poll_end {
        measured
            .intervals
            .between_polls
            .record(started.saturating_duration_since(previous));
    }
    let result = future.poll(cx);
    let ended = Instant::now();
    let interval = match owner {
        PollOwner::Client => &mut measured.intervals.client,
        PollOwner::Server => &mut measured.intervals.server,
    };
    interval.record(ended.saturating_duration_since(started));
    measured.last_poll_end = Some(ended);
    measurement.set(measured);
    result
}
async fn watch_round(
    server: &mut PortalServer,
    client: &mut PortalClient,
    measurement: &Cell<Measurement>,
    phase: WatchPhase,
) -> WatchRound {
    loop {
        let mut client_round = std::pin::pin!(client.next_round());
        let mut server_round = std::pin::pin!(server.next_round());
        tokio::select! {
            round = std::future::poll_fn(|cx| measure_poll(client_round.as_mut(), cx, PollOwner::Client, measurement)) => {
                return round.unwrap_or_else(|error| panic!("actual live client lane progress: {error:?}; phase={phase:?}; intervals={:?}", measurement.get().intervals));
            }
            round = std::future::poll_fn(|cx| measure_poll(server_round.as_mut(), cx, PollOwner::Server, measurement)) => {
                let round = round.unwrap_or_else(|error| panic!("actual live server lane progress: {error:?}; phase={phase:?}; intervals={:?}", measurement.get().intervals));
                assert!(!round.events.into_iter().flatten().any(|event| matches!(event, PortalEvent::Ended)),
                    "original server lifetime must remain live during the watch witness; phase={phase:?}; intervals={:?}", measurement.get().intervals);
            }
        }
    }
}

#[tokio::test]
async fn actual_public_watch_dual_startup_publishes_correlated_retained_pending_status() {
    let mut f = support::repo::RepoFixture::new();
    let expected = f.original.clone();
    let expected_peer = f.server_peer;
    // Explicit trusted registration occurs before either owner starts its original lifetime.
    f.client_repo
        .initialize_originals(&[(
            0,
            expected.clone(),
            SourceMinima {
                event: EventSeq(0),
                store_revision: 0,
                membership_revision: 1,
            },
        )])
        .unwrap();
    let prepared_config = watch_config(&f.client_repo, None);
    let prepared_client = PortalClient::prepare(f.client_repo, prepared_config, 0).unwrap();
    let server_config = config::server_config(&f.repo);
    let mut server = PortalServer::new(f.repo, server_config, Duration::from_secs(5)).unwrap();
    let addresses = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            for event in server
                .next_round()
                .await
                .expect("actual server listening")
                .events
                .into_iter()
                .flatten()
            {
                if let PortalEvent::Ready { addresses } = event {
                    return addresses;
                }
            }
        }
    })
    .await
    .expect("actual three listening addresses, not guessed ports");
    let client_started = Instant::now();
    let conservative_client_until = client_started.checked_add(Duration::from_secs(5)).unwrap();
    let mut client = prepared_client
        .start(addresses, Duration::from_secs(5))
        .unwrap();
    let measurement = Cell::new(Measurement::default());
    let startup = tokio::time::timeout(Duration::from_secs(3), async {
        let (mut authenticated, mut subscribed, mut status) = (false, false, None);
        loop {
            for event in watch_round(&mut server, &mut client, &measurement, WatchPhase::Startup)
                .await
                .events
                .into_iter()
                .flatten()
            {
                match event {
                    WatchEvent::ReceiptAuthenticated { peer } => {
                        assert_eq!(
                            peer, expected_peer,
                            "actual authenticated generated server peer"
                        );
                        authenticated = true;
                    }
                    WatchEvent::Subscribed => subscribed = true,
                    WatchEvent::Status(outcome) => {
                        assert!(status.replace(outcome).is_none());
                    }
                    WatchEvent::Ended => {
                        panic!("original client lifetime ended during authentic startup; phase=Startup; intervals={:?}; client_remaining={:?}; server_remaining={:?}", measurement.get().intervals, conservative_client_until.saturating_duration_since(Instant::now()), server.until.saturating_duration_since(Instant::now()))
                    }
                }
            }
            if authenticated && subscribed {
                return status;
            }
        }
    })
    .await
    .expect("real Noise receipt authentication and signed notification subscription");
    assert!(server.until.saturating_duration_since(Instant::now()) > Duration::from_secs(1));
    assert!(
        conservative_client_until.saturating_duration_since(Instant::now())
            > Duration::from_secs(1)
    );
    let observed = tokio::time::timeout(Duration::from_secs(1), async {
        if let Some(status) = startup {
            return status;
        }
        loop {
            for event in watch_round(&mut server, &mut client, &measurement, WatchPhase::RetainedStatus)
                .await
                .events
                .into_iter()
                .flatten()
            {
                match event {
                    WatchEvent::Status(status) => return status,
                    WatchEvent::Ended => {
                        panic!("original client lifetime ended before retained status; phase=RetainedStatus; intervals={:?}; client_remaining={:?}; server_remaining={:?}", measurement.get().intervals, conservative_client_until.saturating_duration_since(Instant::now()), server.until.saturating_duration_since(Instant::now()))
                    }
                    _ => {}
                }
            }
        }
    })
    .await;
    // This witness executes even when missing Status causes the watchdog to return Elapsed.
    assert!(
        Instant::now() < server.until && Instant::now() < conservative_client_until,
        "both original owner deadlines remain live at the public Status oracle"
    );
    let outcome =
        observed.expect("public watch must automatically publish its correlated retained status");
    let AdmittedReceipt::Status(status) = outcome else {
        panic!("actual pending receipt status");
    };
    assert_eq!(status.request, expected.request());
    assert_eq!(status.account, expected.original().account_id);
    assert_eq!(status.device, expected.original().device_id);
    assert_eq!(status.operation, expected.operation());
    assert_eq!(status.binding, expected.binding_digest());
    assert_eq!(status.phase, ReceiptPhase::Pending);
    assert!(status.current.admits(SourceMinima {
        event: EventSeq(0),
        store_revision: 1,
        membership_revision: 1
    }));
    client.shutdown();
    server.shutdown();
}
