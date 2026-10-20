mod chat_retained_latency_support;
#[path = "chat_peer_delivery_support/mod.rs"]
#[expect(
    dead_code,
    reason = "Shared delivery fixture also has outbox and receipt oracles used by its separate delivery tests"
)]
mod fixture;
#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;

use chat_retained_latency_support::{Clock, populate};
use fixture::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_store::chat::{Channel, ChatStore, HistoryQuery, ReceiverClock};
use nf_transport::{
    auth::ServerPin,
    chat::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, fetch_history},
    node::{PeerServer, PeerServerEvent, request_status},
    query::QueryResult,
    records::{PeerLimits, RetainedPhase},
    session::SessionPolicy,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

// Qualification of the unchanged receiver, not a throughput claim. Each phase
// retains genuinely admitted signed originals AND fresh alias request bindings.
#[tokio::test(flavor = "current_thread")]
async fn retained_originals_and_aliases_leave_actual_control_queries_progressing() {
    let fixture = Fixture::new();
    let clock = Arc::new(Clock::default());
    let mut store = ChatStore::create_with_clock(
        &fixture.receiver_path,
        &fixture.policy,
        &fixture.state,
        Arc::clone(&clock) as Arc<dyn ReceiverClock>,
    )
    .unwrap();
    let pin = ChatPeerPin::from_current(&fixture.state, author(&fixture.bob)).unwrap();
    let whole = Instant::now();
    let mut previous = 0;
    for retained in [0, 32, 128] {
        populate(&mut store, &fixture, &clock, previous, retained, whole);
        previous = retained;
        let verification = Instant::now();
        assert_eq!(store.known_frontiers().unwrap().revision, retained);
        let verification_us = verification.elapsed().as_micros();
        let mut chat = ChatPeerServer::new(store, &fixture.server_vault, fixture.policy).unwrap();
        let address = loop {
            if let ChatPeerServerEvent::Ready { address, .. } = chat.next().await.unwrap() {
                break address;
            }
        };
        // A real, independent control owner shares this single-thread executor.
        // Its signed status request must progress while Chat scans retained rows.
        let scratch = scratch::Scratch::new();
        let community = owned_community::OwnedCommunity::new(&scratch.0);
        let policy = SessionPolicy {
            scope: community.state.scope,
            ruleset: [4; 32],
            content: [5; 32],
            limits: PeerLimits::default(),
            minimum_membership: 1,
        };
        let control_pin = ServerPin {
            peer: libp2p::PeerId::from_bytes(&community.server.public.peer).unwrap(),
            account: community.server.public.account,
            device: community.server.public.device,
            minimum_membership: 1,
        };
        let mut control =
            PeerServer::new(community.server_store, &community.server_vault, policy).unwrap();
        let control_address = loop {
            if let PeerServerEvent::Ready { control, .. } = control.next().await.unwrap() {
                break control;
            }
        };
        let query = HistoryQuery {
            request: RequestId::from_bytes([113; 16]),
            reader: author(&fixture.alice),
            channel: Channel::General,
            after_cursor: 0,
            limit: 1,
        };
        let began = Instant::now();
        let history = fetch_history(&fixture.client_vault, &fixture.state, &pin, address, query);
        let status = request_status(
            community.client_store,
            &community.client_vault,
            policy,
            control_pin,
            control_address,
            RequestId::from_bytes([9; 16]),
        );
        tokio::pin!(history, status);
        let mut page = None;
        let mut control_us = None;
        let mut history_us = None;
        tokio::time::timeout(Duration::from_secs(10), async {
            while page.is_none() || control_us.is_none() {
                tokio::select! {
                    result = &mut history, if page.is_none() => {page=Some(result.unwrap()); history_us=Some(began.elapsed().as_micros());}
                    result = &mut status, if control_us.is_none() => {assert_eq!(result.unwrap(), QueryResult::Status(RetainedPhase::UnknownRequest)); control_us=Some(began.elapsed().as_micros());}
                    event = chat.next() => {event.unwrap();}
                    event = control.next() => {event.unwrap();}
                }
            }
        }).await.expect("retained Chat/control qualification parent fence");
        let page = page.unwrap();
        assert_eq!(page.entries.len(), usize::from(retained != 0));
        assert_eq!(page.next_cursor, u64::from(retained != 0));
        if let Some(entry) = page.entries.first() {
            assert_eq!(entry.receiver_cursor, 1);
            assert_eq!(entry.signed.message.text, "retained public qualification 1");
            assert_eq!(entry.signed.message.author, author(&fixture.alice));
        }
        // The public APIs already enforce these absolute five-second deadlines;
        // record actual elapsed time as well to expose synchronous scan stalls.
        println!(
            "NF_CHAT_RETAINED_MEASUREMENT originals={retained} aliases={retained} verify_us={verification_us} history_us={} control_us={} budget_us=5000000 fixture=public_admission_local_loopback",
            history_us.unwrap(),
            control_us.unwrap()
        );
        assert!(history_us.unwrap() < 5_000_000);
        assert!(control_us.unwrap() < 5_000_000);
        store = chat.into_store();
        assert_eq!(store.known_frontiers().unwrap().revision, retained);
    }
    assert!(
        whole.elapsed() < Duration::from_secs(120),
        "bounded fixture and measurement case"
    );
}
