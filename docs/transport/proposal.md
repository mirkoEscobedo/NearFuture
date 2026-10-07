# NF-019 / issue #23: explicit-peer transport proposal

Historical preimplementation proposal: at the time this proposal was written, root had approved the portable direction and exact initial dependencies, including the narrow maintained Yamux seam below, and only a crate manifest/lib placeholder existed. The proposal text records that original plan; it does not describe current implementation status. See [current implementation evidence](evidence.md) and [foreground usage](run.md) for the implemented and verified portable slice. Shared manifests and dependency lock remain owned by root. Full issue #23 remains open, including native-game failure responsiveness, Pub/Sub capability and semantic history/snapshot synchronization.

## Minimal vertical slice

Two owned foreground Rust processes connect through explicitly supplied IPv4 loopback multiaddresses ending in the pinned actual PeerId. TCP is encrypted/authenticated by maintained libp2p Noise and multiplexed by maintained Yamux. There is no plaintext fallback, mDNS, multicast, relay, DHT, NAT traversal, pubsub or automatic authority election. Each node has separate physical control and bulk listeners/swarms with distinct protocol names, so saturated bulk streams cannot occupy the control connection pool.

The control path negotiates a closed Rust-only protocol, mutually verifies application DeviceProof against current admitted durable membership, and queries real retained status. Network identity alone grants no authority. Stable RequestId survives reconnect and is used only for retained lookup; no writes or strategic commands are registered. Pending and committed rejection can be returned; successful kernel outcomes, missing history and semantic snapshot installation remain Unsupported. Repeated query traffic cannot commit or repeat an operation because this slice has no mutation handler. That is not evidence of transport-driven exactly-once execution.

## Dependencies and effects

Proposed exact production dependencies:

```toml
libp2p = { version = "=0.57.0", default-features = false, features = ["tokio", "tcp", "noise", "yamux", "ed25519", "macros", "request-response"] }
tokio = { version = "=1.53.2", default-features = false, features = ["rt", "macros", "time", "sync"] }
futures = { version = "=0.3.34", default-features = false, features = ["std", "async-await"] }
yamux = "=0.14.1"
getrandom = "=0.4.3"
sha2 = { version = "=0.10.9", default-features = false }
zeroize = "=1.9.0"
nf-contract = { path = "../nf-contract" }
nf-identity = { path = "../nf-identity" }
nf-store = { path = "../nf-store" }
```

Root alone adds the workspace member and updates the shared lock incrementally, preserving accepted pins. The existing isolated scout lock/typecheck establishes dependency availability, not functioning two-peer transport. The request-response 0.30.0 Codec uses return-position impl Future; no async-trait dependency is required. No schema1 foundation Profile payload or opaque kernel domain7 envelope is used. A later typed kernel rejection-code mapping may require an approved direct nf-kernel dependency; the first version can expose only a bounded generic retained rejection with its existing binding digest and durable sequence.

The maintained libp2p-yamux 0.48.0 public wrapper exposes only stream count. Its inner yamux 0.14.1 default connection receive-window ceiling is 1 GiB. Small application frames alone do not change that ceiling. Approved seam: exact direct yamux 0.14.1 and narrow libp2p-core upgrade/StreamMuxer glue around its public maintained Connection, with control max4 streams/1MiB connection window and bulk max1 stream/256KiB window. The glue must not reimplement Noise, cryptography, multiplexing state or TCP. If that seam is not verified, the larger backend-window residual must be explicit and a small total resource claim cannot pass.

## Identity and current policy

A separate owner-private opaque vault blob stores the maintained libp2p Ed25519 keypair encoding. Creation is explicit; startup loads existing material and fails closed if it is missing/corrupt. Encoding buffers are zeroized and never logged. This does not expose nf-identity SecretSeed: the existing device key signs DeviceProof through its public sign operation.

The actual Noise-authenticated PeerId comes from the Swarm event, not an incoming claimed peer/role. Its canonical to_bytes value must equal the current admitted Device.peer, PublicIdentity.peer and DeviceProof.peer. The connection ID similarly comes from the actual per-lane Swarm event. Local vault identity is created/loaded with that actual peer binding. Signed admission/rotation policy owns changes to the mapping.

The client has an explicitly trusted ServerPin (actual PeerId, account, device, scope and minimum known membership frontier). It verifies the server's actual Noise peer matches the pin, the current durable membership owner/account/device mapping matches the pin, the server device is active and its existing PLAYER policy permits the read-only protocol. There is no invented HOST role or promotion to authority. Every server DeviceProof is verified against current durable membership and the bound handshake/query context. Unknown, revoked, stale-frontier or wrong-role server identity is unavailable.

The server verifies the client's DeviceProof with MembershipState.authorize using ProtectedOperation::Economic for retained operation status, requiring current PLAYER membership. It rereads current durable membership before each protected operation; no cached role or merely matching retained record is authentication. Known membership frontiers are monotonic. Missing/quarantined storage, stale membership, failed proof and disconnection surface bounded unavailable/read-only status, never a synthetic acknowledgement.

## Closed record and transcript draft

Protocol names are `/nearfuture/peer/control/1` and `/nearfuture/peer/bulk-verify/1`. They are Rust-only namespaces; their version/capability numbers do not register foundation ControlEnvelope, canonical payloads, Java schemas or kernel domain7.

Framing uses four-byte big-endian body length, validated before allocation. All other integers use little endian. The fixed envelope header is 126 bytes: ASCII `NF-PEER-1` plus NUL (10), version u16=1, kind u8, lane u8 (actual CONTROL=1/BULK=2), app session16, universe16, history16, ruleset32, content32. Session zero is allowed only for initial Hello. Wrong fixed width, nonzero unused padding, extra bytes, unknown kind/version/required capability, role inconsistent with the actual listener, scope or policy mismatch fail closed.

A fixed PeerField is length u8 plus a 128-byte zero-padded canonical PeerId buffer. It must parse as PeerId and round-trip to the exact bytes. DeviceProof uses existing fields in order: scope32, account16, device16, frontier u64, PeerField129, challenge32, signature64 (297 bytes). No private seed appears on wire.

Closed required-capability bit1 means retained-status query; optional bit2 means opaque bulk digest verification. Unknown required bits reject; only the intersection of known optional bits is selected. Initial limits are described below and may only be lowered. No strategic write, kernel-success or semantic sync capability is announced.

| Kind | Lane | Body / state |
| --- | --- | --- |
| Hello=1 | Either | fresh client nonce32, requested capabilities and fixed offered limits; initial state only |
| ServerHello=2 | Either | fresh server nonce32, fresh nonzero app session16, local/selected caps and limits, server DeviceProof; bound to both actual peers and client nonce |
| ClientProof=3 | Either | client DeviceProof over the same context with distinct stage; consumes pending handshake |
| Finished=4 | Either | server DeviceProof with distinct finished stage; client becomes active only after verification |
| BeginQuery=5 | Control | stable RequestId16, fresh client request nonce32; requires active session |
| QueryChallenge=6 | Control | request16, request nonce32, fresh server operation nonce32, current membership frontier u64, bound challenge32 |
| ProveQuery=7 | Control | request16, request nonce32, client DeviceProof; consumes exactly one pending challenge |
| RetainedStatus=8 | Control | request16, operation16, binding digest32, closed phase, optional durable sequence, bounded generic retained rejection; server proof binds reply digest |
| Unsupported=9 | Either | closed bounded reason code, no arbitrary message or payload |
| BeginBulk=10 | Bulk | transfer16, total u64<=1MiB, chunk count u16<=128, whole-byte SHA25632, fresh request nonce32 |
| BulkChallenge=11 | Bulk | fresh one-use challenge and current membership frontier, bound to the whole transfer descriptor |
| ProveBulk=12 | Bulk | DeviceProof, consumes pending transfer challenge |
| BulkChunk=13 | Bulk | transfer16, exact next index u16, length u16<=8192, bytes; only one admitted in-progress transfer |
| BulkVerified=14 | Bulk | transfer16, size u64, digest32 and server proof; digest evidence only, no semantic admission or installation |

Retained statuses are UnknownRequest, Pending and Rejected. A committed successful kernel outcome returns Unsupported instead of a foundation probe success. PeerId, RequestId and connection/session are transport lookup bindings, not newly invented account authority. Full minimal field layouts and code values will be frozen with independent positive/malformed vectors before implementation relies on them.

Application proof challenges are SHA256 of closed explicitly encoded context, never Protobuf reserialization: domain `NF-PEER-AUTH-1\0` plus stage, protocol/lane/version, actual client and server PeerFields, app session, scope/policy, both fresh handshake nonces, selected/offered capabilities and limits, and the proof's membership frontier. Stages distinguish server hello, client proof and server finished. Per-query domain `NF-PEER-QUERY-1\0` additionally binds RequestId, fresh request nonce, fresh server operation nonce and current membership frontier. Reply proof additionally binds the fixed status bytes digest, with a different stage. These values occupy DeviceProof.challenge and the existing nf-identity device_digest/signature algorithm then authenticates them.

Each pending challenge is stored against the exact actual ConnectionId, actual peer, lane, session, scope, request/transfer descriptor digest and nonce, with expiry. First proof attempt consumes it before authorization; replay, wrong connection, reconnect, changed request or expiry cannot reuse it. Connection/session invalidation deletes pending challenges, queued replies and in-progress bulk state. Device authorization is checked again when a protected operation starts; bulk chunks are one admitted transfer, with current membership checked before accepting progress and revoked/changed membership aborting it.

Bulk digest verification remains opaque and background-only. Unknown or unregistered bytes can have a correct digest. No `BulkVerified` response installs a world or acknowledges a journal transaction. Missing-history and canonical resynchronization responses remain explicit Unsupported until a separately reviewed closed decoder/registry can enforce all nested negotiated budgets.

## Proposed limits and public seams

| Resource | Control | Bulk |
| --- | --- | --- |
| Body frame | <=4096 bytes | <=9216 bytes |
| Chunk data | Not applicable | <=8192 bytes |
| Application queued bytes | 64KiB globally reserved | 128KiB globally reserved |
| Application queued items | 16 | 4 |
| Concurrent request streams | 4 | 1 |
| Established connections | 4 globally,1 per peer | 2 globally,1 per peer |
| Pending dial/accept | 2 outbound,4 incoming | 1 outbound,2 incoming |
| In-progress bulk transfer | None | 1 per authenticated bulk session |

Pending proof challenges are globally <=8, expire in 5s and are connection-bound. Request timeout 5s, idle connection deadline 10s, at most 1 explicit address dialed at a time, bounded Swarm event/handler buffers (<=16) and max 4 inbound negotiating streams apply. Foreground process lifetime is bounded in the test fixture. Byte and item quotas are checked before enqueue/allocation; no unbounded retry channel, request-response send loop or diagnostics queue is permitted. Codec reads prefix/body incrementally and rejects trailing input. Unknown selectors/lengths reject before body expansion; no nested user collections exist in this fixed registry. These are application and configured backend bounds, not total process RSS or a frame-latency claim.

Proposed owned APIs (exact signatures to settle with public RED tests):

```text
TransportIdentity::{create_private(vault),load_private(vault),peer_id()}
PeerPolicy { scope, ruleset, content, trusted_server_pin, limits }
PeerCodec::{decode_control,decode_bulk,encode}; fixed closed registry only
PeerServer::{bind(policy,identity,Store),addresses(),poll_event(),invalidate()}
PeerClient::{dial(policy,identity,current_membership,explicit_addresses),request_status(RequestId),poll_event(),invalidate()}
AuthorizedQuery (private construction after actual-peer/fresh-proof/durable policy checks)
PeerEvent::{Ready,RetainedPending,RetainedRejected,UnknownRequest,Unsupported,Offline}
```

All network, authentication, hashing, SQL and waiting remain owned background/node work. Storage has one owner; the transport port cannot manufacture membership, acknowledgement or successful kernel outcomes. The first composition can perform bounded-size read-only SQLite work on the owned node loop, with wall-clock I/O stalls explicitly unbounded and game progress decoupled; a dedicated bounded storage actor is a later choice if actual control latency demands it, not a speculative service graph. No game object, Java callback or worker touching a live object enters this crate.

## Meaningful RED/GREEN and ownership plan

1. Public fixed-codec tests from an independent manual fixture writer: API-missing RED, then GREEN for exact positive widths and malformed/version/scope/role/padding/truncated-overlength declarations. Do not use the production encoder as the only oracle.
2. Real admitted identities, actual generated libp2p PeerIds and real SQLite membership: public authentication/request seam RED, then GREEN for current-device proof, wrong peer/signature/frontier/scope/role, revocation and consumed nonce. No fake game/JAR/API fixtures.
3. Two actually encrypted loopback peers first in owned node tasks, then two foreground child processes under package-native Windows Job Object/Linux process-group supervision. Before real serving exists the process/handshake/status test must fail meaningfully. Private keys stay in real owner-private temporary vaults; readiness exposes only explicit public loopback addresses/peer IDs.
4. Real database retained Pending/Rejected lookup and restart/reconnect with stable RequestId: normal shutdown/reopen preserves binding/sequence, stale session/challenge/reordered proof reject, duplicated queries do not alter the journal. Unknown history/version and successful kernel/snapshot semantics are unsupported. Mutation idempotence is not claimed while mutations are unregistered.
5. Hold/saturate actual bulk connection and partial frames while control returns within the configured headless deadline. Test queue byte/item bounds and response limits, bulk disconnect discard, idle cleanup, failed Noise/application handshake, third peer mismatch and current durable revocation. Child failures/timeout paths kill/reap only exact owned handles; no existing application launch or broad process kill.
6. Author-separated read-only review of frozen source, exact lock/features, fixture independence, private lifecycle, hard quota enforcement and fresh multi-process gates. At most two semantic repair rounds. Public docs retain native-game and semantic-resync exclusions instead of closing #23 on headless evidence.

No implementation or benchmark PASS is asserted by this proposal. The parent coordinator owns final API approval, shared setup, isolated candidate publication and issue status.

