# Foreground peer transport usage

This partial Rust-only peer transport provides authenticated retained-status lookup and opaque digest verification. It installs no game hook, semantic snapshot, remote membership update, strategic command executor, discovery service or Pub/Sub route. Issue23 remains open for those acceptance gates and actual game responsiveness evidence.

Build the foreground binary with `cargo build -p nf-transport --bin nf-peer --locked --offline`. Run `nf-peer serve <existing-private-vault> <saves-root> <existing-database> <owner-config> 12000` for a finite12-second foreground listener, or `nf-peer query <existing-private-vault> <saves-root> <existing-database> <owner-config> <request-id-32-lowercase-hex>`. Serve duration is500..60000 milliseconds. Exactly six arguments follow the executable. No command generates or replaces private identity after a missing/corrupt file, opens a background service, or launches Starsector.

Provisioning is an explicit trusted owner operation through the accepted PrivateVault/identity/Store APIs and `TransportIdentity::create`. A persisted Noise Ed25519 identity is a separate private blob; it is not an NF account/device signing seed and grants no role. The existing NF device's admitted durable membership must bind its actual Noise PeerId. The server must be the explicitly trusted owner with current PLAYER role; clients must be admitted active PLAYER devices. Private files retain the accepted identity vault ACL/owner-mode checks. Protect the vault and trusted owner configuration using the host's account boundary.

The UTF-8 owner config is at most4096 bytes, read through one opened handle with a fixed4097-byte envelope. It is a closed key/value format:

```text
NF-PEER-CONFIG-1
universe=<32 lowercase hex>
history=<32 lowercase hex>
ruleset=<64 lowercase hex>
content=<64 lowercase hex>
event_sequence=<trusted durable u64>
store_revision=<trusted durable u64>
membership_revision=<trusted durable u64>
```

Query configs additionally require `server_peer=<canonical PeerId>`, `server_account=<32 lowercase hex>`, `server_device=<32 lowercase hex>` and `control_address=/ip4/127.0.0.1/tcp/<nonzero-port>/p2p/<exact-pinned-peer>`. Duplicate, unknown, missing and empty values reject. Known durable frontiers protect against opening a rolled-back database; never replace them with incoming peer claims. Public policy hashes are owner-supplied expectations for this headless slice, not a measurement of a live game's installed content.

The server prints one `NF_PEER_READY <control-address> <bulk-address>` only after both actual loopback listeners exist, and prints `NF_PEER_STOPPED` after its finite foreground duration. Query output is one of UNKNOWN_REQUEST/PENDING/REJECTED/UNSUPPORTED. Static errors contain no credential, database path, save data or remote diagnostic text. Successful retained kernel outcomes remain UNSUPPORTED until a named closed payload schema and parity are registered. A status lookup performs no strategic writes or repeat execution.

The public Rust foreground helpers are `request_status`, `verify_bulk`, and `query_and_verify_bulk`. The last owns exactly one Store and two separate physical Swarms; it admits one outstanding request per lane and performs fresh durable membership lookups throughout each proof/reply/progress. Its caller supplies bounded opaque bytes and exact trusted endpoint pins. Verified digest results expose transfer ID, total and hash only; they are not documents, snapshots, history, native projections or durable acknowledgements.

For owned process tests on Windows use the package-native Job Object supervisor rather than launching an unattended daemon. The private verification harness has `package.json` and `.agentic/project.json` with the pinned workspace-template metadata and an exact `node:.` cargo command override. Invoke `npm.cmd exec -- workspace-template inspect .tmp/transport-native`, then `npm.cmd exec -- workspace-template verify .tmp/transport-native --scope module --module node:. --timeout 180000`. The override is `cargo test --manifest-path E:/github/NearFuture/Cargo.toml -p nf-transport --locked --offline`. Tests track each exact child handle, bounded stdout/stderr, output-based listener readiness, explicit deadlines, abnormal kill/reap and graceful finite exit. Their private scratch fixtures are removed inside the test's verified owned directory.

Reproduce the independent public synthetic protocol vectors with `node crates/nf-transport/tools/generate-peer-vectors.cjs --check`. The producer uses Node Buffer/crypto without importing production codecs. The fixture's fixed synthetic signing seeds are public test constants; installed user identities never enter these vectors.

Hard parser limits and negotiated application admission are separate. PeerCodec checks its fixed hard lane frame limit before allocating a body; the foreground owner reapplies negotiated lower frame/chunk limits before record use/enqueue. Queue accounting counts exact encoded body bytes plus a separately documented four-byte framing prefix per item; typed objects and backend buffers are finite residuals, not a strict process heap budget. Maintained Yamux credit tests show backend control bytes progressing while bulk credit stalls, then bulk resuming after reads. The TCP/Noise same-client test proves concurrent protected query and bounded bulk progress; it does not claim that a normal one-request bulk application exhausted its receive window. Total RSS and the game's p99<=2ms bridge budget remain unmeasured.

## Foreground retained receipt watch

Build the additional foreground binary with `cargo build -p nf-transport --bin nf-portal --locked --offline`. Its operator forms are:

`nf-portal init-book <existing-vault> <saves-root> <existing-database> <config>`

`nf-portal serve <existing-vault> <saves-root> <existing-database> <config> <duration-ms>`

`nf-portal watch <existing-vault> <saves-root> <existing-database> <config> <slot> <duration-ms>`

Duration is500..60000 milliseconds and slot is0..7. Book initialization is an explicit trusted owner action that writes the configured immutable originals and prints their book anchors; it is not a strategic submission command. Watch loads that same configured original and exact anchor, performs authenticated retained lookup, and never resubmits the original operation. A sole repository owner drives separate receipt, bulk and notification physical Swarms. Current durable membership, trusted endpoint pins and externally retained minima remain authorization inputs.

Portal config is a separate closed, ordered ASCII `NF-PORTAL-CONFIG-1` format with a trailing newline, at most16384 bytes. It binds mode, scope, policy hashes, current local and server identities/minima, lane addresses and bounded original/anchor rows; it is not the peer config above. The exact admitted format is defined by [the Portal parser](../../crates/nf-transport/src/portal/config/parser.rs) and [row schema](../../crates/nf-transport/src/portal/config/rows.rs). Existing identity/database provisioning remains required; missing private identity is never regenerated.

The server prints `NF_PORTAL_READY` only after all three listeners exist. Watch prints one correlated `NF_PORTAL_STATUS` for an admitted status, including original request, operation, binding and current frontiers, or a closed `NF_PORTAL_UNSUPPORTED` reason. Both finite owners print `NF_PORTAL_ENDED` on normal completion. Notification dirtiness triggers another authenticated retained lookup rather than an execution or outbox acknowledgement.

`serve-staged` and `watch-staged` append one explicit start-file argument to their corresponding forms. They publish `NF_PORTAL_PREPARED` before a bounded30-second inert operator wait; the start file supplies exactly three addresses under a closed `NF-PORTAL-START-1`/`END` frame. No live owner exists during that preparation, and the wait does not renew a started owner deadline. Paths and regular-file/reparse checks follow the cooperative operator filesystem lifecycle, not an atomic defense against same-owner replacement.

This is headless retained observation. It installs no game hook, semantic snapshot, remote command executor or Pub/Sub route, and grants no native-game capability.
