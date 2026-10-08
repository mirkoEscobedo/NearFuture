pub mod portal_process_support;
use libp2p::{Multiaddr, multiaddr::Protocol};
use portal_process_support::{
    ProcessFixture, config::publish_start, process::OwnedPeer, repo::RepoFixture,
};
use std::{
    ffi::OsString,
    net::TcpListener,
    path::Path,
    time::{Duration, Instant},
};
const LIFETIME: Duration = Duration::from_secs(5);
struct Scenario {
    // Exact children drop and reap before fixture SQL/vault files are removed.
    server: OwnedPeer,
    watcher: OwnedPeer,
    fixture: ProcessFixture,
    addresses: [Multiaddr; 3],
    server_until_lower_bound: Instant,
    watcher_until_lower_bound: Instant,
    completion_watchdog: Instant,
}
fn arguments(fixture: &ProcessFixture, watch: bool, config: &Path, start: &Path) -> Vec<OsString> {
    let mut args = vec![
        OsString::from(if watch {
            "watch-staged"
        } else {
            "serve-staged"
        }),
        fixture
            .root
            .join(if watch { "client" } else { "server" })
            .into_os_string(),
        fixture.root.join("saves").into_os_string(),
        fixture
            .root
            .join(if watch {
                "client/policy.sqlite"
            } else {
                "server/policy.sqlite"
            })
            .into_os_string(),
        config.as_os_str().to_owned(),
    ];
    if watch {
        args.push(OsString::from("0"));
    }
    args.push(OsString::from("5000"));
    args.push(start.as_os_str().to_owned());
    args
}
impl Scenario {
    fn start() -> Self {
        let fixture = RepoFixture::new().into_process();
        let reservations: [TcpListener; 3] =
            std::array::from_fn(|_| TcpListener::bind(("127.0.0.1", 0)).unwrap());
        let listeners: [Multiaddr; 3] = std::array::from_fn(|index| {
            format!(
                "/ip4/127.0.0.1/tcp/{}",
                reservations[index].local_addr().unwrap().port()
            )
            .parse()
            .unwrap()
        });
        let addresses: [Multiaddr; 3] = std::array::from_fn(|index| {
            let mut address = listeners[index].clone();
            address.push(Protocol::P2p(fixture.server_peer));
            address
        });
        let (serve_config, watch_config) = fixture.write_configs(&listeners, &addresses);
        let server_start = fixture.root.join("server.start");
        let watch_start = fixture.root.join("watch.start");
        let mut watcher = OwnedPeer::spawn(&arguments(&fixture, true, &watch_config, &watch_start));
        let prepared = watcher.line_before(
            "NF_PORTAL_PREPARED",
            Instant::now() + Duration::from_secs(30),
        );
        let watch_prepared_observed = prepared.as_ref().map(|line| line.1);
        watcher.healthy_running();
        assert_eq!(prepared.unwrap_or_else(|| panic!(
            "Watch preparation is a SETUP prerequisite; observed={watch_prepared_observed:?}; inspected={:?}; {}",
            Instant::now(), watcher.logs())).0, "NF_PORTAL_PREPARED",
            "Watch preparation frame; observed={watch_prepared_observed:?}; inspected={:?}; {}",
            Instant::now(), watcher.logs());
        let mut server =
            OwnedPeer::spawn(&arguments(&fixture, false, &serve_config, &server_start));
        assert_ne!(
            server.id(),
            watcher.id(),
            "two distinct production OS peer processes; watch_prepared={watch_prepared_observed:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        let prepared = server.line_before(
            "NF_PORTAL_PREPARED",
            Instant::now() + Duration::from_secs(30),
        );
        let serve_prepared_observed = prepared.as_ref().map(|line| line.1);
        server.healthy_running();
        watcher.healthy_running();
        assert_eq!(prepared.unwrap_or_else(|| panic!(
            "Serve preparation is a SETUP prerequisite; watch_prepared={watch_prepared_observed:?}; serve_prepared={serve_prepared_observed:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(), server.logs(), watcher.logs())).0, "NF_PORTAL_PREPARED",
            "Serve preparation frame; watch_prepared={watch_prepared_observed:?}; serve_prepared={serve_prepared_observed:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(), server.logs(), watcher.logs());
        drop(reservations); // Port reservation alone is never evidence of a real listening lane.
        let server_until_lower_bound = publish_start(&server_start, &listeners) + LIFETIME;
        let ready = server.line_before("NF_PORTAL_READY", server_until_lower_bound);
        let ready_observation = ready.as_ref().map(|line| line.1);
        server.healthy_running();
        watcher.healthy_running();
        assert!(
            Instant::now() < server_until_lower_bound,
            "conservative server SETUP bound consumed; no behavioral RED; prepared={:?}; ready={ready_observation:?}; server_lower={server_until_lower_bound:?}; inspected={:?}; server={}; watcher={}",
            (watch_prepared_observed, serve_prepared_observed),
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        let (ready, ready_observed) = ready.unwrap_or_else(|| panic!(
            "actual three-listener Ready is a SETUP prerequisite; prepared={:?}; ready={ready_observation:?}; server_lower={server_until_lower_bound:?}; inspected={:?}; server={}; watcher={}",
            (watch_prepared_observed, serve_prepared_observed), Instant::now(), server.logs(), watcher.logs()));
        let expected = format!(
            "NF_PORTAL_READY {} {} {}",
            addresses[0], addresses[1], addresses[2]
        );
        assert_eq!(
            ready,
            expected,
            "actual Ready must equal all three retained addresses and pins; ready={ready_observed:?}; server_lower={server_until_lower_bound:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        let watcher_until_lower_bound = publish_start(&watch_start, &addresses) + LIFETIME;
        let startup = (Instant::now() + Duration::from_secs(3))
            .min(server_until_lower_bound - Duration::from_secs(1))
            .min(watcher_until_lower_bound - Duration::from_secs(1));
        let authenticated = watcher.line_before("NF_PORTAL_RECEIPT_AUTHENTICATED", startup);
        let subscribed = watcher.line_before("NF_PORTAL_SUBSCRIBED", startup);
        let startup_observed = (
            ready_observed,
            authenticated.as_ref().map(|line| line.1),
            subscribed.as_ref().map(|line| line.1),
        );
        server.healthy_running();
        watcher.healthy_running();
        assert!(
            Instant::now() < server_until_lower_bound && Instant::now() < watcher_until_lower_bound,
            "both conservative original runtimes must remain live during SETUP; observed={startup_observed:?}; startup={startup:?}; lower={:?}; inspected={:?}; server={}; watcher={}",
            (server_until_lower_bound, watcher_until_lower_bound),
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        let (authenticated, authenticated_observed) = authenticated.unwrap_or_else(|| panic!(
            "real Noise receipt authentication is a SETUP prerequisite; prepared={:?}; observed={startup_observed:?}; startup={startup:?}; lower={:?}; inspected={:?}; server={}; watcher={}",
            (watch_prepared_observed, serve_prepared_observed), (server_until_lower_bound, watcher_until_lower_bound), Instant::now(), server.logs(), watcher.logs()));
        let (subscribed, subscribed_observed) = subscribed.unwrap_or_else(|| panic!(
            "real subscription is a SETUP prerequisite; prepared={:?}; observed={startup_observed:?}; startup={startup:?}; lower={:?}; inspected={:?}; server={}; watcher={}",
            (watch_prepared_observed, serve_prepared_observed), (server_until_lower_bound, watcher_until_lower_bound), Instant::now(), server.logs(), watcher.logs()));
        assert_eq!(
            authenticated,
            format!("NF_PORTAL_RECEIPT_AUTHENTICATED {}", fixture.server_peer),
            "actual authentication pin; observed={startup_observed:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        assert_eq!(
            subscribed,
            "NF_PORTAL_SUBSCRIBED",
            "actual subscription frame; observed={startup_observed:?}; inspected={:?}; server={}; watcher={}",
            Instant::now(),
            server.logs(),
            watcher.logs()
        );
        // Reader observations follow each owner's first until construction; this bounds cleanup,
        // whereas START timestamps remain conservative lower bounds used only to prove liveness.
        let completion_watchdog = ready_observed
            .max(authenticated_observed)
            .max(subscribed_observed)
            + LIFETIME
            + Duration::from_secs(3);
        Self {
            server,
            watcher,
            fixture,
            addresses,
            server_until_lower_bound,
            watcher_until_lower_bound,
            completion_watchdog,
        }
    }
    fn original_live(&mut self) {
        self.server.healthy_running();
        self.watcher.healthy_running();
        assert!(
            Instant::now() < self.server_until_lower_bound
                && Instant::now() < self.watcher_until_lower_bound,
            "alive original deadlines must precede the missing-observation unwrap"
        );
    }
    fn actual_sent(&mut self) {
        let watchdog = (Instant::now() + Duration::from_secs(1))
            .min(self.server_until_lower_bound)
            .min(self.watcher_until_lower_bound);
        let observed = self
            .server
            .line_before("NF_PORTAL_SUBSCRIPTION_RESPONSE_SENT", watchdog);
        self.original_live();
        let sent = observed
            .expect("actual matching Subscribed response completion must be observable")
            .0;
        let words: Vec<_> = sent.split(' ').collect();
        assert_eq!(words.len(), 4);
        assert_eq!(words[1], self.fixture.client_peer.to_string());
        assert!(words[2].starts_with("ConnectionId(") && words[2].ends_with(')'));
        assert!(words[3].starts_with("InboundRequestId(") && words[3].ends_with(')'));
    }
    fn finish(mut self, expected_status_count: Option<usize>) {
        let completion = self.completion_watchdog;
        assert!(
            self.server.finish_before(completion).success(),
            "{}",
            self.server.logs()
        );
        let watcher_status = self.watcher.finish_before(completion);
        assert_eq!(self.server.count_output("NF_PORTAL_ENDED"), 1);
        if watcher_status.success() {
            assert_eq!(self.watcher.count_output("NF_PORTAL_ENDED"), 1);
        } else {
            // Actual server completion can close the client's connection before its later original end.
            assert!(
                self.watcher.logs().contains("NF_PORTAL_ERROR Offline"),
                "{}",
                self.watcher.logs()
            );
        }
        assert_eq!(
            self.server
                .count_output("NF_PORTAL_SUBSCRIPTION_RESPONSE_SENT"),
            1
        );
        if let Some(count) = expected_status_count {
            assert_eq!(self.watcher.count_output("NF_PORTAL_STATUS"), count);
        }
        self.fixture.assert_original_pending_unchanged();
        for address in &self.addresses {
            let port = address
                .iter()
                .find_map(|part| {
                    if let Protocol::Tcp(port) = part {
                        Some(port)
                    } else {
                        None
                    }
                })
                .unwrap();
            let rebound = TcpListener::bind(("127.0.0.1", port))
                .expect("all three actual server sockets released after peer completion");
            drop(rebound);
        }
    }
}
#[test]
fn production_peer_processes_observe_actual_subscription_response_sent() {
    let mut scenario = Scenario::start();
    scenario.actual_sent();
    scenario.finish(None);
}
#[test]
fn production_watch_renders_correlated_original_pending_without_resubmitting() {
    // Run only after the first actual sent observation slice is GREEN.
    let mut scenario = Scenario::start();
    scenario.actual_sent();
    let watchdog = (Instant::now() + Duration::from_secs(1))
        .min(scenario.server_until_lower_bound)
        .min(scenario.watcher_until_lower_bound);
    let observed = scenario.watcher.line_before("NF_PORTAL_STATUS", watchdog);
    scenario.original_live();
    let status = observed.unwrap_or_else(|| panic!(
        "public CLI Watch must render its correlated retained status; watchdog={watchdog:?}; inspected={:?}; lower={:?}; completion={:?}; server={}; watcher={}",
        Instant::now(), (scenario.server_until_lower_bound, scenario.watcher_until_lower_bound),
        scenario.completion_watchdog, scenario.server.public_context(), scenario.watcher.public_context())).0;
    let hex = |bytes: &[u8]| -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() };
    assert_eq!(
        status,
        format!(
            "NF_PORTAL_STATUS PENDING {} {} {} 0 1 1",
            hex(scenario.fixture.original.request().as_bytes()),
            hex(scenario.fixture.original.operation().as_bytes()),
            hex(&scenario.fixture.original.binding_digest())
        )
    );
    scenario.finish(Some(1));
}
