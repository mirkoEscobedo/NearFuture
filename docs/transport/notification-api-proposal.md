# Notification and three-lane portal API proposal

Documentation-only source/API ownership proposal after independent NF-NOTIFY-1 contract R1 design PASS. The frozen contract is [notification-contract.md](notification-contract.md), raw SHA256 `f3e818776bb3ee9fe0c660424bc420f938e852c3f7d3ba0eec51466440aa0dce`. Phase A's [receipt-contract.md](receipt-contract.md) is independently reviewed, revised wording SHA256 `763fbd65d5d6df42268c6b30d3ddaf3bea23972303fa7b0ab5f618ab58f31efd`. This proposal changes neither accepted source nor those fixed records/transcripts. Production ownership begins only on root's later dispatch after whole issue36 acceptance.

## Source ownership

Bootstrap owns future new `src/notification/**`, later new `src/portal/**`, `tests/notification_*.rs`, `tests/portal_*.rs` and new dedicated `tests/portal_support/**`. Existing accepted tests/support and control1/bulk1 source remain frozen. A future separate finite `src/bin/nf-portal/**` avoids expanding accepted `nf-peer` CLI behavior. Root controls its target registration and public exports. Notification fixtures/producer stay independently owned: guidance produces Phase A vectors first, then root assigns Phase B vectors; bootstrap consumes them and does not substitute Rust output as their expected bytes.

Runtime owns new `receipt/**`, its tests and the single `ReceiptRepository`. Root owns shared `lib.rs`, additive `TransportIdentity` builders, manifests/lock/features, workflow and architecture. Any requested repository method is authored inside `receipt/**` by runtime after agreement; bootstrap does not cross-edit it. Neither accepted Store/identity nor kernel record registries need changes. Notification and receipt use their separate reviewed namespaces; `records::Lane` stays the accepted Control/Bulk enum, with no invented Lane3 accepted by old decoders.

Suggested notification files are `mod.rs`, `records/{mod,model,encode,decode}.rs`, `limits.rs`, `transcript.rs`, `handshake.rs`, `broker.rs`, `client.rs`, `queue.rs` and `network.rs`. Split only when responsibilities require it, not speculative empty modules. The broker owns fixed subscription/projection/dirty-marker/awaited state and no repository. The client owns fixed subscription/sequence/dirty-generation/follow-up state and no repository. The closed shape decoder cannot produce an authorization certificate. Actual event context is required at every handshake/operation step.

Suggested portal files are `mod.rs`, `server.rs`, `client.rs`, `connection.rs`, `poll.rs`, `bulk_network.rs`, `owner_command.rs` and finite CLI parsing. Server and client each take one existing repository by value at construction; they cannot accept a database path or open/reopen another repository inside a handler. Their structs hold that one repository, three owned Swarms, at most one connection state per lane, one receipt state, one existing BulkServer/BulkClient state and one notification state. Trusted original descriptors/recovery anchors remain owned by the repository. No standalone notification task holds a second Store, cloned writable handle, private key copy or cached membership grant.

## Stable repository ports requested from runtime

The following are proposed `pub(crate)` ports rather than new externally callable strategic APIs. Names may be settled before implementation, but each semantic boundary must remain explicit. Existing `OriginalReceipt` retains the independently derived immutable RequestBinding; it is the client-side source of selector data.

```rust
// Private value wrappers for accepted u64 storage/membership revisions.
struct MembershipRevision(u64);
struct StoreRevision(u64);
// Read-only owned values, not persisted permissions.
struct ReceiptSelector {
    scope: Scope,
    account: AccountId,
    device: DeviceId,
    request: RequestId,
    operation: OperationId,
    binding: [u8; 32],
}
enum ProjectionPhase {
    Pending,
    Rejected { sequence: EventSeq },
    Committed { sequence: EventSeq },
}
struct RetainedProjection {
    selector: ReceiptSelector,
    phase: ProjectionPhase,
}
struct ProjectionRead {
    membership_revision: MembershipRevision,
    current_event: EventSeq,
    current_store: StoreRevision,
    projection: RetainedProjection,
}

impl ReceiptRepository<'_> {
    fn local_public(&self) -> &PublicIdentity;
    fn selector(&self, original: &OriginalReceipt)
        -> Result<ReceiptSelector, PeerError>;
    fn observe_selector(&mut self, selector: &ReceiptSelector,
        expected_membership: MembershipRevision)
        -> Result<ProjectionRead, PeerError>;
    fn with_current_read<R>(&mut self,
        f: impl for<'a> FnOnce(CurrentRead<'a>) -> Result<R, PeerError>)
        -> Result<R, PeerError>;
}
```

EventSeq is the existing nf-contract counter. Store and membership currently use u64; the proposed private transparent wrappers distinguish their values without new public schemas or wire fields. Selector fields are private. The client constructor verifies its actual local account/device/scope, configured pin/profile and recomputed RequestBinding digest using `OriginalReceipt`; no incoming notice sets expectations. The server's closed selector admission verifies equivalent fields from decoded BeginSubscribe against the fresh retained lookup. `observe_selector` uses `Store::query_bound` and fresh current membership/known frontiers in one synchronous owner step with no await or intervening owner mutation. It requires the exact expected active membership revision and request's original principal/device/operation/digest. Unknown, different binding, unavailable/quarantined policy or different principal returns a finite refusal; it never exposes another user's projection. Projection values do not authorize a later send: that send reloads current policy separately.

`CurrentRead<'a>` is an ephemeral private credential/policy borrow with read-only access to the freshly validated `MembershipState`, exact local `PublicIdentity`, and existing private `SecretSeed` device signer. It exposes no Store, SQL connection, raw key bytes, write callback, path or Debug credential output. Its higher-ranked synchronous closure returns a lifetime-independent `R`, so a signer/policy borrow cannot escape into an async task or queued state. The repository loads and validates actual SQL membership/frontiers plus local device key/peer and configured server ownership before calling the closure. The closure may perform bounded closed protocol authentication/signing, return a typed record or session state, and must not await, invoke an owner command, clone secret seeds or call arbitrary user callbacks. Returned owned membership observations are never saved as grants.

This borrow is required to reuse accepted `ServerSession::begin` and `BulkServer::{begin,prove,chunk}` unchanged: those public seams already take fresh MembershipState and, when signing, `&SecretSeed`. Receipt and notification state machines likewise verify their current actual event binding and exact transcript inside that synchronous read cut. Every protected stage obtains a new cut. Before queued record admission/send or Ack acceptance the portal obtains another current read and fences a changed policy revision. A response prefix remains the exact already signed bytes; it cannot be rewritten to a new epoch. This port grants no role and cannot manufacture a DurableAck. If runtime/root prefer avoiding the signer borrow, the equivalent narrower port is a family of repository-owned typed authentication/bulk-step adapters; do not return an unguarded public secret getter or mutable Store callback instead.

Fresh observation and signer borrow are proposal ports, not current implemented APIs. Consistent `known_frontiers` and membership capture uses accepted single-owner synchronous seams; a stale observation's matching number alone is not a permission. The caller must provide the actual Swarm PeerId/ConnectionId/session at the state-machine method, and only a current checked matching epoch can sign/admit its result. The repository does not accept a remote `now`, host flag, role or caller-selected policy update. Monotonic timing/nonces remain owner-derived.

## Shared identity/network builder ports requested from root

```rust
impl TransportIdentity {
    // Same loaded Noise identity; no regeneration or raw key export.
    fn build_receipt_lane(&self)
        -> Result<Swarm<ReceiptBehaviour>, PeerError>;
    fn build_notification_lane(&self)
        -> Result<Swarm<NotificationBehaviour>, PeerError>;
    fn build_portal_bulk_lane(&self)
        -> Result<Swarm<PortalBulkBehaviour>, PeerError>;
}
```

Root's methods clone the already loaded private libp2p key only inside the existing trusted identity implementation and delegate to each owned closed builder. Runtime owns ReceiptBehaviour/codec network constructor; bootstrap owns NotificationBehaviour and new PortalBulkBehaviour constructor. No record chooses a builder. Additive builders enforce the contract's tighter one established/one pending incoming/one pending outgoing connection per lane, stream limits4/1/1, and exact request-response protocols. Existing `build_lane` and its control1/bulk1 callers stay unchanged. The portal's new bulk builder uses the existing PeerCodec and maintained LaneMuxConfig for Bulk, but its own connection-limit behaviour; it must not claim global3 while silently retaining accepted bulk's established maximum2.

Receipt and notification builders use their own exact codecs and transcript profiles. A pending upgraded mux remains included in conservative all-stage accounting until source/actual tests prove it cannot poll before occupying an established slot. The caller cannot configure larger ceilings or optional protocols. Three actual addresses/readiness events are required before the portal reports ready; all addresses contain the exact loaded/pinned PeerId. No guessed adjacent ports or discovery fallback exists.

## Notification state-machine shape

`NotifyActual { peer: PeerId, connection: ConnectionId }` comes only from the owner event dispatch, never wire fields. A closed `NotifyPolicy` contains trusted scope/ruleset/content/server pin and bounded offered limits, without device keys. `NotifyServer`/`NotifyClient` methods receive the exact decoded record, actual event tuple, current read and owner monotonic Instant; the portal derives those arguments from actual state. Constructors admit only the exact notify/1 selected protocol. One-use stage consumption happens before shape/peer/signature checks according to the fixed contract.

Proposed server methods are `begin_handshake`, `client_proof`, `begin_subscription`, `prove_subscription`, `observe_projection`, `next_notice`, `accept_ack`, `expire` and `invalidate`. Each returns a finite typed transition/action rather than accepting closures that access Store. For example `observe_projection` compares a fresh fixed projection with its baseline and marks dirty; `next_notice` returns a checked signed Notice only after token/queue reservation and current policy admission. `Subscribed` creates the active baseline only after exact retained selector validation, and outbound Notices wait for its actual ResponseSent permit release.

Proposed client methods mirror handshake/subscription stages, then `admit_notice`, `next_query`, `query_started`, `query_completed`, `expire` and `invalidate`. `admit_notice` returns a bounded Ack plus a dirty generation; it does not return a strategic receipt. `next_query` returns the unchanged `OriginalReceipt` already retained by the repository and a private generation token. The portal begins a fresh receipt operation through runtime's receipt state/recovery ports, rather than reusing a notification proof. Query completion clears only the generation it covered; a newer dirty marker schedules at most one subsequent query. Current policy failure or disconnect drops notification sequence and schedules explicit reconnect/manual query according to the independently owned receipt recovery book. No notification restores active proof/session state.

Queue reservations carry a private owner-instance/ticket and encoded body count. Portal connection maps are bounded to actual one connection and one awaited operation, with fixed-size response/expiry state. Matched flushed/completed events release only their own permit; unmatched events cannot free another lane/owner ticket. Foreground lifetime and protocol expiry are separate private monotonic cutoffs. The portal polls at most one ready application event from each lane per rotating round, performs deadline/policy sweep, then yields. A ready notification queue cannot monopolize control or bulk; no detached spawn or endless unbounded drain is introduced.

## Trusted local mutation for the real transition oracle

The network handler exposes no `FnMut(&mut Store)` and no write command in any reviewed codec. Actual server test Commit requires a distinct named trusted owner seam, authored with runtime after agreement:

```rust
// Constructible only from trusted local setup, never a record decoder.
struct TrustedPreparedCommit { /* closed accepted CommittedBatch */ }
impl ReceiptRepository<'_> {
    fn commit_trusted_prepared(&mut self, prepared: TrustedPreparedCommit)
        -> Result<DurableAck, PeerError>;
}
```

Trusted test setup prepares a real accepted profile1 operation before repository ownership transfers, derives its valid batch through accepted kernel logic and seals exactly that prepared operation/before frontier into this command. The synchronous method reuses `Store::commit` and returns its real post-transaction Ack; stale/wrong batch refuses. It is not called by Hello/Subscribe/Notice/Ack/Receipt/Bulk record processing and creates no default strategic executor. A finite test harness can deliver this one preconfigured local command after notification baseline admission; the command source is trusted owner setup, not peer JSON, arbitrary SQL or raw closure. Production CLI's initial serve/client modes expose no commit command. If a later trusted operational CLI mutation is required, that separate owner policy/source slice needs review.

After actual Commit the same repository reads retained projection; only a changed real projection dirties the subscription. No test replaces the retained lookup with a fake phase or fake Ack. Pre-Commit crashes/errors can leave Pending; receipt recovery resolves uncertainty. A post-Commit lost reply does not manufacture another debit/event. Store outbox acknowledgement is never called by portal notification or receipt code.

## Finite CLI and test ownership

Future `nf-portal serve` accepts explicit existing private vault/save root/database/trusted config and duration500..60000ms, no key generation/roles/fork/autocreate. Its new PortalConfig names explicit typed notification/control2/bulk endpoints only where needed and remains bounded/closed; accepted nf-peer configuration stays unchanged, and its final grammar is a later concrete root-reviewed CLI patch. `nf-portal watch` selects one already retained `OriginalReceipt` book slot/anchor, authenticates all three independently pinned physical lanes, subscribes, fresh-queries and may verify one explicitly bounded opaque bulk source. It requires the original account/device identity; successor recovery cannot relabel it. It may report public hint/receipt/digest metadata, never native state or StrategicAck. Duration/readiness/output/owned exact-child cleanup remain bounded and fail closed on absent or corrupt identity/config/book.

Tests use new dedicated support, actual disposable owner-private vaults, maintained signatures, one repository per process, real accepted profile1 Store operation and exact pinned addresses. No edits to accepted support or copies of secret-bearing installed files. A separate fixed test fixture command can seal the one trusted prepared batch; readiness waits for actual all-three listeners. Real two-process tests cover notification->fresh committed receipt, current membership/connection/session fences, first failed attempt consumption, rate/refusal/coalescing/generation correctness, reply loss/reconnect, and adversarial finite notification flood while control2 receipt and opaque bulk progress. The flood may close the notification lane early; the assertion is bounded refusal and other-lane progress, not64 successful deliveries. No synthetic backend claim substitutes for measured credit/stream saturation.

Source ownership and ports must be agreed with runtime/root before code. Notification behavioral RED begins only after a compiled valid seam exists; all structural failures are setup, not behavior. Freeze independently generated Phase B vectors first, preserve accepted control1/bulk1 regressions, then author/native/hosted evidence and independent review gate this slice. It does not close semantic replica recovery or full issue23/native acceptance.

## Settled internal agreement before standalone source dispatch

Runtime and bootstrap agreed the repository owns ReceiptSelector and ProjectionRead in `receipt/model.rs`. The reserved ports are `local_public`, client-only `selector(&OriginalReceipt)`, `observe_selector` at an exact expected membership revision, and synchronous higher-ranked `with_current_read`. ProjectionRead returns the exact original selector, Pending/Rejected/Committed phase and fresh current event/store/member values. A separate server selector admission constructor requires the currently authenticated remote principal/device/scope plus the decoded request/operation/binding; it cannot use the client-local constructor to relabel a subscriber. The active notification state must supply that authenticated principal from its own actual connection/fresh checked policy, rather than accept a wire host flag.

TrustedPreparedCommit is sealed from trusted local setup with the exact accepted prepared batch, before-state hash, source revision and selector. It consumes once; the sole repository performs the real Store::commit and reports its Ack only afterward. There is no mutable Store getter/callback or wire conversion. Actual source signatures for the separate remote principal type will settle with runtime's receipt module before portal integration; this outstanding type naming is not permission to construct fake admitted policy in a test.

Root dispatched only the new notification pure model/codec/transcript and dedicated tests first. Portal, repository integration and finite CLI await a later dispatch. The public pure values are named NotifyLimits, NotifyContext, NotifyBody, NotifyRecord, NotifySelector and NotifyHandshakeTranscript/NotifySubscribeTranscript/NotifyNoticeTranscript. NotifySelector is untrusted fixed wire shape and is distinct from the repository's admitted ReceiptSelector. Pure decoding, transcript hashing and manual roundtrip tests establish no SQL, peer authentication, replay, subscription or operational authority acceptance.

Vertical implementation order is valid fixed shape/closed limits; independent frozen Node vector consumer for all record and transcript bytes; signed prefix and namespace/stage/descriptor mutation tests; then separately dispatched actual one-use/current-policy state with real signatures; then sole-repository portal and owned two-process trigger/flood/control/bulk evidence. Absent exports, missing fixtures/ports and lint/dependency setup do not count as behavioral RED. Each compiled intended behavior failure is recorded before its minimum repair, and already green unrelated accepted tests remain unchanged.
