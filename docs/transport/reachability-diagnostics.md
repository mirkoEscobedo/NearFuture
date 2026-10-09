# Explicit loopback query diagnostics

This is a partial diagnostic slice of [issue #24](https://github.com/mirkoEscobedo/NearFuture/issues/24). It does not complete that issue or its #23 dependency.

The existing control transport accepts an explicitly configured, pinned IPv4 loopback TCP endpoint. `request_status_diagnosed` accepts `Option<Multiaddr>` and returns the original query result with bounded diagnostic codes. The existing `request_status` API delegates to it and retains the five-second query deadline and authorization/persistence behavior.

An absent endpoint returns `Offline` with `NotDialed` and `NoConfiguredPeer`. Explicit endpoint validation rejects encoded addresses longer than 256 bytes, relay circuits, unsupported protocol shapes, remote IPv4 routes, zero TCP ports, and peer-pin mismatches before any dial. The only admitted shape is `Ip4(loopback)/Tcp(nonzero)/P2p(exact pin)`.

Observations record this attempt: `DialAttempted` follows accepted `swarm.dial`; `DirectConnectionObserved` follows the actual connection and expected peer/single-connection checks; `ReplyValidated` follows a successful authenticated query reply. They do not grant membership, request authority, or prove ongoing connectivity. Existing key/vault, membership, role, account/device pin, session/challenge and connection checks remain in the actual request path.

Failures distinguish seven closed dial codes, connection closure, request-response failure, the existing deadline, endpoint refusal, and an existing typed operation error. Diagnostic codes exclude raw endpoints, peer IDs, storage paths and nested transport/denial strings. The endpoint scan uses bounded iteration rather than collecting an address-sized vector.

Existing control-lane physical limits remain four established connections, one per peer, four pending incoming and two pending outgoing connections, four concurrent request streams, 16-event/handler buffers, four negotiating inbound streams, dial concurrency one, five-second request/query deadlines and a ten-second idle timeout. This slice changes none of those limits and does not qualify broader application byte/resource budgets.

## Qualified local evidence

Validation used author base `ec4af3e8fc025252f07a2ac781c4872d8d6f5b18`, Windows Rust 1.98.1, locked/offline Cargo and one native owner. The frozen listening-without-Noise first test produced a genuine RED at `Uninstrumented != DialAttempted` after `Offline` passed; its `Deadline` assertion was then unreached. The same unchanged test passed all three assertions on GREEN.

The final unchanged four-case foreground harness passed with zero failures, ignored or filtered tests in 22.33 seconds (29.840-second managed phase, including 4.82-second compilation). It checks an actual authenticated reply, a live non-Noise listener reaching the query deadline, refusal of a remote endpoint before dialing, and absent configuration staying offline. Twelve unchanged endpoint/dial/privacy controls passed with zero failures, ignored or filtered tests. Workspace format check and strict Clippy for the library plus those two test targets passed without warnings. Every successfully completed qualification phase used 240-second Windows Job limits, concurrency one, restored the three process environment keys exactly, and left no owned processes. Original gate refusals, RED, and the repaired `collapsible_if` lint failure remain preserved in local evidence.

The let-chain style repair retained the predicates and short-circuit fallback assignment; no existing foreground case isolates assignment of `Failure::Operation`. The full four-case harness was therefore rerun. Unchanged pure control sources retained their accepted twelve-case result without another runtime run.

## Remaining issue #24 scope

This transport has no installed relay reservation, relay circuit dial support, DCUtR upgrade, remote route, discovery, or CGNAT handling. Direct/relay-only/upgrade/unavailable-relay scenarios, quota enforcement, relay privacy policy and revocation behavior still need their own implementation and runtime evidence. A configured route or a connection observation cannot stand in for that evidence. No hidden mandatory server or automatic relay/resource policy is introduced by this slice.

The fresh default `main` comparison supports a narrow draft: the author base is its ancestor and these seven paths do not overlap intervening changes. Runtime qualification remains for the author source above; the merged current Main state has not been executed by this slice.
