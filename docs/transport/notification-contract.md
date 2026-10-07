# NF-NOTIFY-1 bounded receipt-hint contract

Documentation-only Phase B proposal for issue #23, following [portable-sync-plan.md](portable-sync-plan.md). No codec, capability, repository, listener or CLI is implemented by this document. Phase A's separate [receipt-contract.md](receipt-contract.md) must be reviewed before authoritative receipt queries are used. This proposal preserves accepted NF-PEER-1 and the closed foundation/Java/kernel registries. It closes neither semantic synchronization nor native responsiveness gates.

## Ownership and meaning

The broker is an explicitly addressed two-peer application publisher/subscriber over a third physical TCP/Noise/Yamux connection, using the existing pinned libp2p request-response feature and maintained dependencies. Protocol `/nearfuture/peer/notify/1` has its own closed codec. The explicit notification address includes the exact pinned PeerId and belongs to that third listener; no inferred port or record-supplied address is accepted. It enables no gossip, forwarding, discovery, multicast, compression or remote strategic command route. Actual accepted listener/Swarm events select lane 3; a record cannot select its endpoint by claiming a lane.

The initial broker serves only the accepted Store1 retained-request profile used by Phase A. MiniatureStore/profile2 needs a separately reviewed single-owner repository integration; this document does not silently enable it. One foreground repository owner holds exactly one Store and polls all three Swarms. Notification code borrows narrow current-membership and retained-query access from that owner; it cannot open another Store or retain mutable database access. The server's exact Noise PeerId/account/device and current durable membership owner must match the configured server pin. Both devices must be current, unrevoked, peer-bound PLAYER members. Every signed operation uses the existing Economic authorization predicate as a read gate; the name grants no economic write. No new role is created. A changed membership revision fences all affected lane sessions, even if keys, roles or numeric request status look unchanged. Remote claims never initialize local policy or replace protected known minima.

The only topic is `RequestReceiptAvailable` (tag 1). One active subscription names one exact RequestId, OperationId and immutable binding digest belonging to that subscriber's original account/device/scope. Subscribing cannot relabel another principal's request. Unknown or mismatching retained bindings are refused, without exposing another request's status. The subscriber already has the original binding from its local request owner; a received hint cannot create that binding.

A Notice says only that the subscriber should re-query. It contains no phase, event sequence, kernel outcome, resource balance, world snapshot or spendable authority. The client re-queries through Phase A control2 with a new operation nonce/challenge/proof and its exact original RequestId/OperationId/binding, pinned server and current protected minima. Notice delivery, NoticeAck, duplicate loss, coalescing and expiry cannot commit, replay or acknowledge a strategic transaction. In particular NoticeAck calls neither Store outbox acknowledgement nor StrategicAck. Notification sequence is ephemeral lane state, not EventSequence, StoreRevision or a durable request sequence.

## Closed framing and namespace

Outer framing is u32BE encoded-body length, admitted before allocation. The body contains exactly one record, with no trailing bytes. All other integers are little endian. Fixed header 128:

```text
NF-NOTIFY-1 NUL (12)
version u16=1, kind u8, actual lane u8=3
application session16, universe16, history16, ruleset32, content32
```

Hello alone has zero application session; every later record has the exact fresh nonzero session. Scope/ruleset/content must equal trusted configuration. IDs and operation nonces are nonzero; unknown kind/version/lane/topic, noncanonical presence or trailing bytes close this lane with a finite local error. Shape decoding authorizes nothing. This header is intentionally a separate codec from NF-PEER-1 and Phase A NF-PEER-2; neither accepted header nor capability registry widens.

PeerField and DeviceProof are exactly the accepted fixed 129-byte and 297-byte layouts: peer length u8 in1..128, zero-filled canonical PeerId prefix; proof universe16/history16/account16/device16/frontier u64/PeerField129/challenge32/signature64. Proof scope/account/device/actual peer are checked independently against the active binding and SQL membership.

NotificationLimits is 14 bytes: maximum body u16, queued body bytes u32, queued items u16, notices per second u16, burst u16, pending operation challenges u16. Valid values are respectively 768..1024, maximum 16384 and at least one maximum body,1..16,1..8,1..8, exactly 1. Hard-default values are1024/16384/16/8/8/1. Selected limits are exact componentwise minima of independently valid offered/server values and must themselves validate. Rates use the monotonic token bucket described below. Required capability is exactly bit 1 in this notification namespace; available and selected are exactly 1, optional is 0. Unknown bits reject. Capability1 grants only bounded receipt hints, not a receipt, bulk transfer, history or authority.

| Kind | Direction and body fields in exact order | Body bytes | Total with header |
| --- | --- | ---: | ---: |
|1 Hello|subscriber account16/device16/client nonce32/required u32/optional u32/offered limits14|86|214|
|2 ServerHello|server nonce32/available u32/selected u32/server limits14/selected limits14/server DeviceProof297|365|493|
|3 ClientProof|subscriber DeviceProof297|297|425|
|4 Finished|server DeviceProof297|297|425|
|5 BeginSubscribe|subscription16/topic u8/request16/operation16/binding32/client operation nonce32/minimum membership u64/lifetime seconds u16|123|251|
|6 SubscribeChallenge|subscription16/client operation nonce32/server operation nonce32/current membership u64/challenge32|120|248|
|7 ProveSubscribe|subscription16/client operation nonce32/subscriber DeviceProof297|345|473|
|8 Subscribed|subscription16/topic u8/request16/operation16/binding32/first notice sequence u64/lifetime seconds u16/server DeviceProof297|388|516|
|9 Notice|subscription16/sequence u64/request16/operation16/binding32/notice nonce32/server DeviceProof297|417|545|
|10 NoticeAck|subscription16/sequence u64/notice prefix digest32/admitted u8=1/subscriber DeviceProof297|354|482|

Subscription lifetime is 1..30 seconds; the server accepts exactly the requested valid duration. Its private deadline starts at final subscription admission before signing Subscribed; the subscriber uses its own stricter deadline starting immediately before BeginSubscribe. Neither peer accepts a wire timestamp or resets that deadline on response receipt; setup taking the entire local lifetime refuses subscription. First notice sequence is 1. One subscription is created per notification connection; replace, renew and unsubscribe require closing that connection and starting a fresh handshake. No separate heartbeat or remote diagnostic record exists. Pre-authentication and invalid/unsupported requests close the notification lane, rather than producing an authoritative status. A request-response exchange carries Hello/ServerHello, ClientProof/Finished, BeginSubscribe/SubscribeChallenge, ProveSubscribe/Subscribed, then server-initiated Notice/subscriber NoticeAck. There is at most one awaited request on this lane, in either direction; no Notice starts before Subscribed is flushed and its permit released.

## Signed transcripts and one-use contexts

SHA256 is maintained library hashing over exact fixed bytes, never semantic reserialization. The existing maintained Ed25519/device_digest signs DeviceProof. These purpose domains cannot be substituted with receipt, bulk or foundation proofs. Actual PeerFields come from Noise events. Local ConnectionId is retained privately in every pending/active binding; it is not a portable peer-selected wire label.

Handshake challenge hashes 617 bytes:

```text
NF-NOTIFY-AUTH-1 NUL (17)
stage u8 (serverHello1/clientProof2/serverFinished3)
version u16=1, actual lane u8=3
selected protocol digest32 = SHA256(ASCII /nearfuture/peer/notify/1)
actual subscriber PeerField129, actual server PeerField129
subscriber account16/device16, server account16/device16
application session16, universe16/history16, ruleset32/content32
client nonce32, server nonce32
required u32, optional u32, server available u32, selected u32
offered limits14, server limits14, selected limits14
proof membership frontier u64
```

The protocol digest is computed from the exact compiled ASCII protocol name (no terminating NUL) actually selected on the notification endpoint. A different negotiated name/version fails before signing, even if all record fields match; incoming protocol claims cannot select it. Neutral context digest hashes the same order with stage 0/frontier 0. Stage0 is only a context hash, never an admissible signed authorization purpose. ServerHello, ClientProof and Finished use distinct signed stages. Client verifies its configured server pin/current policy before active subscription state exists. Server and client both reload their actual sole local repository's current validated membership at each protected stage. Their admitted current revisions must agree; separately admitted policy disagreement fails closed until trusted policy handling resolves it.

Subscribe operation challenge hashes 244 bytes:

```text
NF-NOTIFY-SUB-1 NUL (16)
stage u8 (subscriberOperation1/serverSubscribed2)
neutral handshake context digest32
subscription16, topic u8=1, request16, operation16, binding digest32
client operation nonce32, server operation nonce32
current membership frontier u64, subscriber minimum known frontier u64
requested lifetime seconds u16
reply prefix digest32 (zero at stage 1)
```

The stage 2 digest is SHA256 of the exact admitted header 128 and Subscribed fields preceding its final DeviceProof (91 bytes). Outer length and proof bytes are excluded. Server response must echo the exact selector/duration/first sequence. Before signing ProveSubscribe, the client checks the actual endpoint/connection/session, exact returned subscription/client nonce, nonzero server nonce, current frontier/minimum and recomputed stage 1 challenge against its retained Begin selector. Unauthenticated or stale challenges never solicit a device signature. Begin creates one private challenge keyed to actual peer/ConnectionId/session/selector/nonces/minimum/current revision; it expires5 seconds after issue. The first ProveSubscribe attempt takes/removes it before checking record shape, timing, signature or retained binding. Wrong input burns that challenge and closes the notification lane. Failure cannot recover or reuse its proof. At final subscription admission, current SQL lookup must again verify actual original request account/device and exact operation/binding. Subscription captures the exact handshake membership revision, private issue time, lifetime and neutral context digest.

Each Notice/Ack proof challenge hashes 212 bytes:

```text
NF-NOTIFY-NOTICE-1 NUL (19)
stage u8 (serverNotice1/subscriberAck2)
neutral handshake context digest32
subscription16, sequence u64, request16, operation16, binding digest32
fresh notice nonce32, current membership frontier u64
prefix digest32
```

Stage1 prefix digest is SHA256 of exact Notice header 128 plus fields preceding its proof (120 bytes). Stage2 is SHA256 of exact NoticeAck header 128 plus fields preceding its proof (57 bytes). The Ack also contains the original Notice prefix digest and must echo it exactly. Thus stage 2 binds both the exact acknowledged Notice and exact Ack result; stage 1/2 cannot be interchanged. Context/selector/frontier/nonces come from the active subscription and sole pending notice, not a received substitute. Signature byte shape alone grants nothing.

The server reloads current membership and exact retained binding before enqueuing, signing and sending each Notice, and before accepting Ack. The receiver does the same current membership/principal/peer/session checks before bounded hint admission and Ack signing. Both require unchanged session membership revision and the appropriate current Economic read authorization. The receiver cannot check the server's private retained database; it checks its locally retained immutable request binding and schedules authoritative control2 query. A notice has no authoritative claim about that database's outcome.

Client accepts only the exact next sequence and one pending notification request; first malformed/wrong/duplicate sequence attempt closes and invalidates its subscription. Sequence advances only after valid bounded admission. Server assigns an increasing sequence only when removing a queued dirty marker to create the sole awaited Notice. Its first Ack attempt consumes the pending notice before validation. Lost/invalid Ack, outbound request failure,5s expiry or checked sequence overflow closes the notification subscription. No retransmission reuses an old request, signature, nonce or sequence. A replacement connection/session/subscription starts at 1 with fresh IDs/nonces and cannot accept old values. Current ConnectionId must match actual events at every step; matching PeerId alone is insufficient.

## Dirty trigger, coalescing and client follow-up

Subscription creation reads the exact retained request status through the sole owner and records a fixed-size baseline: original operation/binding and retained phase plus optional sequence/rejection. No raw kernel/native payload is retained. The client performs a fresh Phase A query after Subscribed, regardless of any notices, to cover a terminal state that existed before subscription or changed during setup.

After a local trusted owner operation actually returns and retained state is readable, or at a bounded100ms poll of the one active selector, the broker compares that exact retained projection to its baseline. It marks dirty only on a real retained transition, including Pending to committed/rejected. Unrelated StoreRevision, outbox acknowledgement, membership metadata or a repeated unchanged query cannot manufacture a new outcome hint. Baseline advances to the actually observed projection when dirty is recorded. Lost/expired hints need no durable event reconstruction: reconnect always fresh-queries the original request. The publisher has no incoming remote operation trigger and cannot synthesize kernel success. A finite test-only owner command may perform a real accepted Store transition under explicit provisioned test policy through the same owner, never a second connection or forged Ack.

Coalesce only an unsent dirty marker for this exact subscription/request/operation/binding. The marker stores no terminal answer. An awaited Notice is immutable. Repeated callbacks for the same observed terminal projection neither alter it nor create a second terminal notification. The accepted immutable request has at most one Pending-to-terminal transition, so a legitimate subscription cannot produce an unlimited stream of new notices. A fixed queued slot and fixed awaited slot bound even adversarial scheduling; no new terminal state is invented to exercise those slots. Coalescing loses hint multiplicity intentionally, while immutable receipts remain queryable. The sole-selector slice therefore reaches at most one queued marker plus one awaited Notice; the general16-item ceiling is an upper admission ceiling, not permission for16 subscriptions, selectors or in-flight requests.

Receiver admission records at most one local dirty flag for this binding and sends NoticeAck only if that bounded slot is available. It schedules at most one fresh control2 query; an already running query may set one follow-up dirty flag. A completed query clears only the dirtiness covered by its own start generation, so a Notice received during that query cannot be erased accidentally. Query failure remains Offline/Pending and retains bounded dirty status; it never invents Committed. Notification loss cannot suppress reconnect/manual query of an unresolved retained binding.

No hint is persisted as canonical request truth. Close/revocation/expiry clears subscription, queued markers, outstanding request IDs, permits and ephemeral dirty sequence; the local original request remains. Restart accepts no restored active session, proof, challenge or subscriber position. A fresh proof may re-query the same immutable request. Lost notices or duplicated delivery cannot charge resources or advance an EventSequence.

## Hard resources, fairness and shutdown

Notification encoded-body ceiling is 1024 before allocation. Queues charge exact encoded bytes before insertion plus four framing bytes per item separately. Each direction is <=16 items/16384 body bytes, one awaited delivery and one pending subscribe challenge; sole-selector occupancy is tighter as above. Immutable awaited bytes remain charged until matching request-response completion; response reservations release only after the exact flushed ResponseSent or explicit owned-lane invalidation. Failure to reserve closes/refuses this lane without dropping an accepted authoritative control receipt. Typed state, proofs and maintained parser buffers are separate bounded residuals.

Outbound and inbound notice token buckets each start with burst <=8, replenish at selected rate <=8/s from private monotonic elapsed time, cap tokens at burst, and use checked integer arithmetic. Backward monotonic observations fail closed. Enqueue/coalescing cannot bypass the outbound send bucket. Inbound over-rate records are refused before additional queue/proof work and fence the notification lane; refused rate input produces no signed Ack. Fixed1024 framing and maintained per-connection backend caps still apply before application rate admission; this is not a claim to rate-limit all Noise/Yamux parsing or guarantee total RSS.

Every challenge/awaited notification expires after 5 seconds; subscription expires after its accepted 1..30s private lifetime. Unsent dirty markers older than 5 seconds may be discarded, requiring a later fresh query rather than fabricated delivery. Lifetime never extends on a Notice, Ack, duplicate or coalescing. Closed lane has no renewal. The notification connection has a5s handshake/subscription-setup deadline; once subscribed its active idle fence is the fixed accepted lifetime. It does not inherit the control/bulk10s idle fence or extend its lifetime on activity. Timeout/clock/backpressure error invalidates only its owned session/connection and exact permits; membership policy change additionally fences the other affected lanes.

This composition explicitly tightens connection admission for the supported one-server/one-client owner: each physical lane permits at most one established connection, one pending incoming and one pending outgoing attempt, with per-peer established maximum 1 and dial concurrency1. Across three Swarms: <=3 established TCP/Noise/Yamux connections, <=3 pending incoming and <=3 pending outgoing attempts, no second subscriber. Pending negotiation has 5s owner deadline. These counts include unauthenticated established connections; speculative extras are refused/closed, not saved in an unbounded list. This is an additive change requiring review; accepted control's4 and bulk's2 backend-established ceilings cannot remain while reporting a global maximum 3. Connection accounting must include established and pending states during transitions without double-admitting replacement peers.

Maintained request-response maximum concurrent streams is control4, bulk1, notification1. Yamux maximum streams and retained inbound buffer are control4/4, bulk1/1, notification1/1; configured connection receive credit is respectively 1048576/262144/262144 bytes. Thus configured established receive credit totals 1572864 bytes, and stream/buffer ceilings total6/6. Established per-connection event and notify-handler buffers remain capped16 each (aggregate 48 each), inbound stream negotiation maximum 4 per connection (aggregate 12), with notification's single request-response slot still authoritative. Request-response timeout is 5s. These are backend/configuration quantities, not total allocation proofs.

Maintained yamux0.14.1 parses and temporarily allocates up to 1MiB per physical connection before stream-credit validation. The three fully established connections require conservative parser allowance <=3MiB, in addition to receive credits, queued encoded bodies, typed objects, encryption/negotiation state and backend event buffers. Until source inspection and a measured admission test prove that a pending upgrade cannot poll Yamux before it consumes an established slot, conservatively count all six pending attempts as additional potentially upgraded parser instances: up to 9MiB parser transient globally, with their additional configured stream/credit/event residuals. Never report3MiB as a complete all-stage bound merely from the established connection limit. Incoming >1MiB Yamux frame fails maintained parser; incoming notification body>1024 fails its length receiver before body allocation. Pending TCP/Noise negotiation residuals are distinct and bounded by the fixed six pending attempts, and remain additional to the conservative all-stage Yamux allowance. If all three admission states per lane could hold a configured mux, conservative configured receive-credit totals 4718592 bytes, stream/buffer ceilings18/18, and per-connection event/handler buffer ceilings144 each; those quantities may be tightened only with observed ownership/admission evidence. Total process RSS and game-frame p99 remain unmeasured.

Control queue ceiling 65536/16, bulk131072/4 and notification16384/16 give a combined per-direction encoded-body ceiling 212992 bytes and item ceiling 36, plus144 framing bytes when full. The sole awaited operation on each lane is a tighter operational cap; quotas may only negotiate down. Globally at most three operation challenges are pending: one per actual lane and below the accepted pending-challenge hard8. All completed/rejected backend events drain into fixed-size state rather than unbounded historical maps. The three-lane owner must reserve a bounded poll opportunity for control, bulk and notifications every round (at most one ready application event per lane in rotating order), run a deadline sweep, then yield. A perpetually ready notification lane cannot win an unbounded select loop. Crypto/SQL work remains synchronous headless work with no hard realtime promise.

Foreground shutdown stops new subscriptions and dirty triggers, invalidates all owned app sessions/challenges/queued hints, closes only actual owned connections and polls their maintained close progress within a fixed deadline. No PID-name kill, remote process control, background service or game launcher is introduced. Tests and CLI children require exact owned handles, private scratch containment, readiness-based deadlines, kill/reap on every failed path, and Windows Job Object supervision. A timeout may discard hints; it may not claim durable acknowledgement or release another owner instance's permit.

## Required implementation evidence before acceptance

Implement only after root approves this contract and exact API/source ownership. Keep the notification codec/transcripts and three-lane owner additive; no approved frozen Store/identity profile is edited implicitly. Suggested owned modules are `notification/{records,auth,broker,client,limits}.rs` plus an explicit reviewed network/owner composition update. A private broker holds only current subscription/fixed projection/marker/in-flight state; public notice-admitted result exposes the bound selector/sequence, never a strategic Ack. Existing exact maintained dependencies suffice. Shared Cargo/lock/features/workflow changes remain root-owned.

Independent Node Buffer/crypto producer proposed at `crates/nf-transport/tools/generate-notification-vectors.cjs`, with public vectors `docs/transport/vectors/notify-v1.tsv`. It imports no Rust codec or generated expected output. Freeze exact widths, raw encodings, transcript hashes and DeviceProof signatures for all 10 record kinds,617/244/212-byte transcripts, exact prefix digests and both signed purposes. Use only explicitly public synthetic test seeds, never installed keys. Include independent malformed records for every unknown tag, exact-one length/trailing violation, zero/nonzero session, wrong actual lane/peer, nonzero padding, field width, lower negotiated cap, selector substitution, sequence/stage replay and membership/minimum mismatch. Root registers mandatory reproducibility only after review; this document creates no vector authority.

Meaningful public RED/GREEN must first compile a valid three-lane application seam. Missing APIs, absent files or dependency/fixture setup failures are not behavioral RED. The real two-process test uses one existing private identity/current signed membership per process and exactly one server Store owner. Its finite trusted server owner prepares an actual accepted profile1 operation before subscription so the selector is already bound, then commits it after baseline subscription admission; subscriber receives a dirty Notice, admits it, fresh-queries a Phase A committed receipt for the exact original binding, and confirms repeated query has no second debit/event/outbox acknowledgement. Wrong original principal/binding cannot subscribe. Lost Notice/Ack, physical server/client reconnect, old connection/session/proof, expiry, current signed revocation and unchanged numeric status must refuse or re-query without manufactured success.

The saturation test starts the same two owned processes with all three actual TCP/Noise/Yamux lanes. Run a bounded authenticated adversarial notification producer (for example64 attempted notices in <=1s, exceeding burst/rate; test input deliberately violates the normal transition-only broker) with raw replay/sequence variants. Receiver must bound queues/refuse or close its notification lane while an exact fresh control2 receipt completes and opaque ordered bulk bytes finish with the expected total/digest. At least one initial legitimate notification must come from a real retained transition; a forged outcome or fake authority is not a flood oracle. Assert current membership/actual peer at admission, all declared high-water counters, no notification-induced Store/outbox writes, bounded shutdown and no owned children left. Also repeat the actual retained-transition callback/poll while its one Notice awaits Ack, proving no duplicate terminal hint is manufactured, and race that legitimate Notice with a control query, proving a later dirty generation survives. A legal single immutable request cannot supply many terminal transitions; receiver flood cases use explicitly adversarial signed inputs rather than fabricated durable transitions.

Backend tests separately measure notification stream saturation/receive-credit stall and resume with maintained Yamux, oversized frame declarations before body read, aggregate connection ceilings and round-robin progress. Do not label a normal short TCP notification as a measured receive-window exhaustion. Report actual elapsed bounds/hardware and application/backend counters, not fabricated RSS or game p99 evidence. Fresh Windows native-supervised and hosted Linux process/ACL/runtime checks, independent author review and at most two semantic repair rounds gate acceptance of this portable slice. Full issue #23 remains open for semantic history/replica recovery, native gameplay hooks and responsiveness.
