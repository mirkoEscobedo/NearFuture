# Portable local IPC checkpoint (NF-006 / issue #10)

This checkpoint connects a foreground Rust node to bounded Rust and Java loopback clients. It does not install a Starsector hook. Issue #10 remains open: real load/save/exit binding, game-thread ownership evidence, snapshot resynchronization, and real-game dead/slow/saturated-node responsiveness have not been measured.

## Observed portable behavior

`nf-ipc-node` publishes an owner-private ephemeral descriptor outside the supplied saves root. The descriptor selects distinct IPv4 loopback control and bulk listeners, a fresh nonzero runtime session, scope/content/ruleset hashes, trusted local account/device, limits and a fresh 32-byte secret. Discovery validates the private vault's ownership/ACL, wrapper digest and fixed descriptor structure before opening a socket. Publication uses create-new; collisions fail closed. Normal foreground shutdown invalidates sessions before closing connections and removing the publication. An abnormal kill can leave an owner-private stale descriptor; attach fails authentication or connection rather than replacing it or killing a process.

Neither endpoint sends the secret over the socket. The separate [local authentication protocol](auth-protocol.md) requires a fresh client and server nonce and three domain-separated HMAC-SHA256 proofs. The client verifies the endpoint before sending its proof and verifies the final proof before activation. Proofs bind the precise role, listener port, runtime session, scope, policy hashes, principal and offered/selected limits. Token authentication grants only the trusted owner's local IPC scope; it is not remote account or device-key authentication.

Control requests are closed foundation protocol-1 records. Intent execution is `Unsupported`. The durable `StoreQueryPort` pins identity from the private vault, rereads current durable membership, checks device revocation, keys, peer, account PLAYER role and a monotonic membership frontier, then reads the retained request binding. Pending and committed rejection statuses are available. Successful kernel outcomes remain `Unsupported`: private kernel domain 7 is not registered on public Rust/Java wire, and probe schema 1 cannot carry a strategic outcome. No query acknowledges a new journal commit.

Bulk transport is an ordered bounded stream with incremental SHA-256 over the transferred bytes, not Protobuf serialization. At most one transfer is in progress per authenticated bulk connection; duplicates/out-of-order chunks and mismatched metadata abort it. The full transfer is capped at the lesser of negotiated transfer bytes and the closed document ceiling of 1 MiB. `DigestCertificate::verify_bytes` returns borrowed opaque digest-verified bytes and binding metadata. It performs no canonical decode, semantic admission, authorization or installation. Arbitrary/unregistered bytes can have a valid digest. A future closed decoder must enforce negotiated limits through nested semantic payloads and temporary allocations before resynchronization or installation can be enabled. The executable's bulk port refuses staging; the test-only staging consumer installs nothing.

## Resource and ownership boundaries

Rust `FramePump` is background-only. It supports incremental 4-byte big-endian framing, rejects declared overlength before body allocation, bounds each poll's reads/writes and retains one outbound frame. A pending write is flushed without consuming a following request. Pipelined requests remain unread in the socket until their prior response drains; they are not silently consumed or dropped. Response admission enforces the same authenticated negotiated wire limits as incoming requests. A trusted QueryPort must construct already bounded data; response validation cannot undo allocations made by an arbitrary custom port.

The default limits are control frame 16 KiB, chunk field 8 KiB, queued bytes 256 KiB, queued items 32, decoded allocation accounting 16 KiB, collection items 256, nesting depth 16 and total transfer 1 MiB. Framing and decoder byte budgets are distinct. The node reserves half the configured queue bytes/items for each endpoint and computes control connection capacity conservatively from two frame buffers plus 4 KiB scratch; defaults admit three control connections and one bulk connection. Separate listeners reserve control progress while bulk is occupied. The standalone `PriorityQueues` offers bounded independent control/bulk queues for future composition; the current node uses its per-connection single outbound slot and TCP backpressure instead.

Unauthenticated inactivity expires at two seconds and authenticated inactivity at five seconds. `NodeServer::poll` performs bounded socket work, but a trusted port can perform SQLite work; the entire call belongs to the background authority, never the campaign/frame thread. These bounds are algorithmic caps, not a p99 timing measurement. Borrowed digest verification can hash up to 1 MiB and belongs to the background too.

Java `BackgroundChannel` owns no threads or game references. It performs bounded nonblocking socket turns and accepts prepared frames. `FrameBridge` is a fixed-capacity SPSC value queue; the producer constructs immutable `FrameMessage` payloads in the background. The campaign consumer polls a bounded number of values and drops stale sessions through `SessionFence`. Single producer/single consumer and lifecycle ownership are required contracts. Actual campaign thread IDs, save boundaries and per-frame latency remain unobserved. The [capture proposal](../capture/proposal.md) remains shadow-only and uses private in-process value types.

## Foreground commands

All arguments shown below are supplied by the trusted owner composition. No command detaches a service, launches the game, controls an existing process, writes a save binding or deploys a mod.

```text
nf-ipc-node init VAULT_ROOT SAVES_ROOT
nf-ipc-node serve VAULT_ROOT SAVES_ROOT NAME UNIVERSE_HEX HISTORY_HEX RULESET_HEX CONTENT_HEX ACCOUNT_HEX DEVICE_HEX DURATION_MS
nf-ipc-node serve-store VAULT_ROOT SAVES_ROOT NAME DATABASE UNIVERSE_HEX HISTORY_HEX RULESET_HEX CONTENT_HEX PEER_HEX KNOWN_EVENT_SEQUENCE KNOWN_STORE_REVISION KNOWN_MEMBERSHIP_REVISION DURATION_MS
nf-ipc-node attach VAULT_ROOT SAVES_ROOT NAME
nf-ipc-node query VAULT_ROOT SAVES_ROOT NAME REQUEST_HEX
```

Duration is 100–60,000 ms. IDs are exactly 16 bytes and hashes 32 bytes in hexadecimal; PEER_HEX encodes 1–128 bytes. `serve` provides authentication and explicit Unsupported responses only. `serve-store` opens an existing database at trusted known frontiers, checks its ruleset, and derives principal from the existing private identity; it never creates a membership or key from client input. Content policy is supplied by trusted composition because it is not an existing durable kernel field. A production integration must pin that value from its verified runtime/content manifest, not an incoming request.

The name has one exclusive cooperative owner-local lifecycle until close completes. Other trusted users of the same vault must not remove, replace or reuse that name concurrently. The current exact-byte guard rejects a replacement already present at close, but comparison and path unlink are separate operations; this is not an atomic ownership primitive against concurrent same-account vault mutation. Use separate unique names for independent lifecycles. No same-account adversarial security guarantee is claimed.

## Remaining game gates

The actual adapter must persist only minimal universe/history/content binding and recognized version/frontier data. Keys, tokens, sockets, workers, service graphs and live game objects must never enter saves. Before load/save/exit or content/history change, the campaign owner must invalidate capture and IPC fences, discard queued applies, and create a fresh runtime session. It must establish thread ownership and pass only bounded immutable value certificates to workers. No implementation in this checkpoint proves those live hooks.

Run matched-save baseline/control/shadow scenarios using [performance methodology](../performance/README.md), including warmup/repeats, game speed and pause state, long frames, allocation/GC, and bridge p99 <=2 ms. Dead node, slow node, queue saturation, partial frames, failed proof, loading a different save and restarting either process all require licensed-game evidence. Use [the debugging guide](../starsector-debugging.md) for classloader, logging and UI hazards. Portable Java 17 compilation/socket success does not prove in-game classloader compatibility or responsiveness. See [reproducible evidence](evidence.md).
