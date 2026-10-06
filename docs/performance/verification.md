# NF-008 tooling verification and remaining blocker

Checkpoint: 2026-10-07. This is a tooling delivery under issue #12's unavailable-reference-data contract. **Actual game baseline/control/shadow runs and instrumentation overhead are unmeasured; issue #12 must remain open.** The implementation revision is supplied by the coordinator's integration evidence.

| Check | Observed result |
|---|---|
| `node --test tests/performance/*.test.mjs` | 18 passed, none failed/skipped. Queue saturation/privacy, missing limits, warm-up/nearest-rank tails, incomplete bridge groups, duplicated frame IDs, matched variance, control identity, instrumentation order, trace loss, exact CLI hashes, clock drift and actual synthetic JFR capture/export. |
| Broad Node tool suite (architecture/build/runtime/performance) | 33 passed before the final thread-identity isolation addition; fresh full regression follows coordinator integration. |
| `npm run check:types` | Strict no-emit checkJs passed over new ESM/JSDoc code and repository tools. |
| `javac --release 17 -Xlint:all -Werror ... ExportRecording.java JfrCapture.java` | Passed on host JDK 21.0.8; target Java 17. |
| Synthetic fixture and exporter on bundled game `jre/bin/java.exe` | Passed on Azul Java 17.0.10, producing six sanitized records: three synthetic frames, three sleeps. This started a short owned synthetic JVM, never Starsector or its existing process. |
| `node tools/performance/cli.mjs saturate --offers 1000000` | Recorded JSON shows 4,096 retained records / 449,450 queued payload bytes, 995,904 shed records, then complete drain. Bridge p99 remains a 2,000 µs budget with `measured: false`. |

Meaningful vertical RED/GREEN cycles preceded implementation: missing buffer and analyzer seams; incomplete bridge groups falsely lacked unavailable status; repeated-comparison module absent; instrumentation-off input order hid a shadow mismatch; incomplete queue limits silently disabled enforcement; actual JFR exporter source absent; supported-control manifest differences rejected; known telemetry loss failed to block comparisons; clock alignment seam absent; duplicate frame IDs inflated evidence; actual game metadata accepted without hardware/live-flag identities; equal thread IDs from different processes incorrectly merged CPU counters. Each focused behavior reached GREEN before the next family grew.

The million-offer driver measures its own Node CPU/RSS and reports them as synthetic observations; it is not an in-game timing or resource guarantee. The JFR integration test uses the real public JFR recorder/reader, not a mock game. Generated classes, raw recordings and temporary reports stay under ignored/temp directories. No game binaries, save files, stack dumps, private host paths or credentials were added to public evidence. The synthetic fixture intentionally includes a test-only private string and proves it disappears from export.

Actual game execution remains unavailable because there is no qualified game UI/process-lifecycle harness or verified whole-frame/campaign instrumentation seam. Closure requires repeated exact-identity late-game reference/control/shadow recordings, a save-safe supported control, measured instrumentation overhead, thread/lock/I/O/GC correlation, and independently reviewed results. No Rust speedup or more-RAM diagnosis is inferred from the tooling.

The trace queue bounds serialized retained payload and item count; whole-process RSS includes allocator/runtime overhead. IPC/JFR values in configuration remain experiment budgets. The JFR exporter permits only supported event fields, caps recording/scanned/exported counts and tells the caller to discard partial output on failure. Time alignment uncertainty and missing observations remain explicit. Runtime captures cannot certify full game content, mutation coverage, save durability or authority.
