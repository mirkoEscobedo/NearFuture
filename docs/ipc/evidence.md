# IPC checkpoint evidence (2026-10-07)

The portable implementation is reviewable; full NF-006 / issue #10 acceptance is incomplete. Licensed-game lifecycle, save contents, live thread ownership, dead/slow-node rendering and p99 bridge <=2 ms are unmeasured. No game/mod deployment, game launch, save modification or native actuator was performed. Independent author-separated review remains required before publishing this partial slice.

## Source and runtime identity

Reviewed implementation revision: `fb40be51d3aa13b1ad97f8944624611bf8847164`, based on accepted foundation plus the merged reference slice. [source-checkpoint.json](source-checkpoint.json) records relative source paths and SHA-256 values of the normalized staged Git source bytes. The implementation revision and independent review are recorded here; hosted OS gates remain pending. Public fixture key/nonce/identity bytes are synthetic deterministic test data. No private token, real key, save or proprietary game binary is committed.

Rust compiler is pinned 1.98.1, Node 24.11.0, Gradle 8.14.3, Java compiler/JVM 21.0.8 and maintained Protobuf generator/runtime 4.31.1. IPC Rust dependencies use exact maintained hmac 0.12.1, sha2 0.10.9, prost 0.13.5, getrandom 0.4.3 and zeroize 1.9.0, plus the accepted local contract/wire/identity/store crates. Rust uses the coordinator's locked workspace. Java compilation targets release 17. The installed game JVM tested here is Azul Zulu 17.0.10+7; this is a headless/socket test, not an in-game classloader or hook test. Installed provenance is in [runtime documentation](../runtime/README.md).

## Observed commands

```powershell
cargo test -p nf-ipc -p nf-wire --locked --offline
cargo fmt -p nf-ipc -p nf-wire -- --check
cargo clippy -p nf-ipc -p nf-wire --all-targets --locked --offline -- -D warnings
node protocol/vectors/generate-local-auth-fixtures.cjs --check
java/gradlew.bat -p java syntheticCheck --offline --no-daemon -PtestJavaExecutable=H:/Games/Starsector/jre/bin/java.exe
java/gradlew.bat -p java :ipc:exportAuthTestClasspath --offline --no-daemon
$env:NF_IPC_JAVA='H:/Games/Starsector/jre/bin/java.exe'
$env:NF_IPC_JAVA_CLASSPATH_FILE='E:/github/NearFuture/java/ipc/build/auth-test-classpath.txt'
cargo test -p nf-ipc --test java_process --locked --offline -- --ignored
```

Observed focused aggregate: 18 IPC tests and 15 wire tests passed, with the explicit Java process test ignored in the default lane. A final second bulk verification regression subsequently passed (2 bulk tests), bringing the reviewed candidate to 19 standard IPC tests plus 1 explicit Java interoperability test. Formatting and all-target warning-denying Clippy passed after that addition. The Java public synthetic gate passed under the actual bundled Java17. The separate real-socket Java test passed in 13.91 seconds: both control and bulk clients returned exactly AUTH_SOCKET_OK, the owned Rust node exited normally, and its exact private publication disappeared. That explicit test requires both environment variables; absence when invoked is a hard failure. See [Java author's detailed evidence](java-auth-evidence.md).

The standard suite includes real loopback partial/slow/EOF framing, two pipelined control responses, saturated separate bulk/control endpoints, bounded streaming staging/abort, negotiated outgoing limits, exact nonce/proof replay/reflection/impostor/context checks, private no-clobber publication and sequential replacement ownership, and foreground process restart with fresh session and authentication. The durable process test creates a real private identity/membership and SQLite Pending operation, closes the database owner, opens it in the foreground query process, queries it over mutually authenticated loopback and confirms Pending remains retained after shutdown. A separate durable test commits a rejection and then revokes the device through the same trusted storage owner, proving each query rereads membership. Successful kernel outcome projection is explicitly Unsupported.

Test-owned child processes use exact handles, hidden Windows creation and bounded wait/kill/reap cleanup. No process name kill, existing application attach or detached background service is used. The node duration is bounded and both CLI and Java handshake deadlines are finite. Logical clock advancement in the inactivity test exercises cleanup branches; it is not a runtime duration or performance measurement.

## RED / GREEN evidence and limits

- Framing and independent bounded priority queue public seams were driven from failing tests before implementation. Lower declared lengths/quotas reject before body allocation.
- Missing `nf_wire::decode_local_auth` was a public API RED; GREEN admits only the separate auth envelope. Independent Node-generated raw vectors and HMAC/transcript expectations prevent the production codec from acting as its own oracle.
- The server pipelining regression was RED with a Disconnected result when its pending-write branch consumed a following valid frame. GREEN uses a write-only flush and delivers both replies.
- `foreground_store_query_reads_retained_pending_after_database_restart` was RED because the CLI had no `serve-store` branch and exited before publication. GREEN uses accepted durable storage/identity ports and actual sockets; its phase is the public Pending enum.
- `trusted_query_response_must_also_obey_authenticated_negotiated_field_budget` was RED when a valid synthetic Profile probe larger than the negotiated 64-byte field cap was emitted. GREEN validates outgoing wire under negotiated limits before writing; the invalid composed response closes the connection.
- Missing `AuthenticatedSession::admit_response` was RED. GREEN rejects old runtime, wrong retained history/request and an invalidated fence; the CLI uses it for returned status.

Mutual proof implementation and fixture-driven security coverage were completed together rather than claiming every proof function had a prior meaningful RED. The public fixtures independently verify all three proof stages. The verified-only bulk boundary deliberately removed full canonical decoding because the available canonical decoder could not honor all negotiated nested semantic allocation budgets. Its certificate now grants only exact borrowed digest/length/session evidence, and malformed/unknown bytes gain no installation authority.

## Required independent/native follow-through

Independent read-only review must cover this exact source and root-owned shared wiring. The private rendezvous name has one exclusive cooperative lifecycle; compare/unlink is not atomic against concurrent same-owner replacement. Existing sequential replacement is rejected; concurrent mutation of a live publication name is unsupported. An atomic cooperative lifecycle guard is future work.

The negotiated wire decoded budget is conservative preflight accounting. It is not total process RSS and does not bound every temporary allocation in foundation semantic Profile conversion/re-encoding or an arbitrary trusted QueryPort. Those operations are finite under foundation hard ceilings and must remain in the background. No claim of a 16 KiB total heap or 2 ms frame latency is made. Full snapshot semantic resynchronization and kernel schema registration/parity require separate reviewed admission work.

Before closing #10, implement and observe real verified load/save/exit hooks, minimal binding serialization and save inspection, exact campaign/worker thread evidence, stale-result rejection across actual save/process transitions, and matched-save latency/allocation/GC/responsiveness scenarios with failed proof, partial peer, dead/slow node and saturation. Capture remains shadow-only, and affected authoritative operations stay pending/read-only. Never encode private kernel domain 7 into foundation probe schema 1 or treat auth capability 2 as a kernel schema registration.

## Independent review checkpoint

Root's author-separated R1 review of the separate Rust auth protocol, nf-wire admission and independent Node fixture generator returned scoped PASS with no important finding; fresh 15 wire tests and fixture reproducibility passed. Bootstrap's independent R1 review of IPC discovery/lifecycle/framing/server/store query/bulk returned scoped PASS with no important finding. It matched all 59 recorded source hashes, passed fresh formatting/all-target strict Clippy, and used package-native Windows Job Object supervision to pass all 20 IPC tests including the explicit Java17 interoperability test. Fresh measured test durations were interoperability 14.08 s, foreground restart 16.14 s and durable Pending restart query 6.69 s. These are test durations, not bridge latency measurements.

Both reviews retain the documented partial-slice limits: cooperative exclusive descriptor name lifecycle rather than atomic concurrent replacement; conservative wire preflight accounting rather than total semantic heap/RSS; opaque digest evidence rather than snapshot installation; no successful kernel outcome projection; and unobserved actual-game lifecycle/save/thread/performance gates. The coordinator owns isolated candidate publication and composed Windows/Linux gates. Full issue #10 acceptance remains open.

The isolated accepted-baseline-plus-IPC candidate passed clean offline npm installation and the full lane: 128 standard Rust tests, 39 Node checks, formatting, warning-denying all-target Clippy, types, boundaries, fixture regeneration and synthetic Java checks. A separate actual Java17 control/bulk process test passed in13.88s, and the locked offline workspace release build passed. All four structured verification steps (full lane, Java classpath export, explicit Rust/Java socket test, release build) returned status0, timedOut=false and processOwnership=windows-job-object through the package-owned native verifier. The default suite's ignored interop test is explicitly invoked; omission does not count as evidence.

Hosted CI now mandates the same explicit two-endpoint socket test on Windows and Linux using its configured Java21. That runtime differs from the separately observed bundled Java17; no CI17 claim is made. Whole issue #10 remains open for the native and broader semantic gaps above.
