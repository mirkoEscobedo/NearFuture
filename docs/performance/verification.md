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

## Opt-in observer delivery verification

Observer source was composed on Main `d4bdc6a3ea47aafe8ed250142513e7191dd51cd2` (tree `9e2ca55fde6fab8b911a7f0e4e2218bf5eaedcec`). Exact delivery revision and hosted results must be recorded in the pull request before merge. This is local licensed API host evidence, not live campaign scheduling, save serialization or a measured game baseline.

Pinned host compiler/runtime: JDK 21.0.8, emitting Java 17 classes with `--release 17 -Xlint:all -Werror`. The selected licensed installation was Starsector 0.98a-RC8 with pinned local API dependencies. Its installed Azul 17.0.10 game JVM did not run the observer. Libraries, private recordings and generated jars were kept outside the public source payload.

| Command on the composed source | Actual result |
| --- | --- |
| `npm ci --offline --ignore-scripts --no-audit --no-fund` | Passed; four vendored packages installed. |
| `npm run check:types` and `npm run check:boundaries` | Passed. |
| `npm run test:tools` | All 87 passed, including real JFR exporter/callback and unavailable-adapter controls; zero skipped/cancelled. |
| `java -classpath java/gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain -p java gameAdapter --no-daemon` | Passed; actual licensed adapter compilation/jar/build, 6 executed tasks and 1 cached contract compile. All 3 plugin/observer classes inspected as major 61 (Java 17). |
| `node --test --test-concurrency=1 tests/game/observer-lifecycle.test.mjs tests/game/observer-after-save.test.mjs tests/game/observer-controls.test.mjs tests/game/observer-save-failed.test.mjs` | All 6 passed in 4.398s; no skips/filtering/cancellations. |

The six contracts cover [registration/event export/detachment](../../tests/game/observer-lifecycle.test.mjs), [successful-save reattachment](../../tests/game/observer-after-save.test.mjs), [invalid opt-in, repeated load and stale callbacks](../../tests/game/observer-controls.test.mjs), and [failed-save reattachment](../../tests/game/observer-save-failed.test.mjs). The failed-save contract first failed after actual compilation/JVM execution with resumedCount 0 versus 1, then the same unchanged whole contract passed after the five-line callback override. Previous first/load/save contracts and their helper sources remain unchanged.

Every local phase used the qualified Windows Job supervisor, 240-second management limit, separate postguard, exact restoration of nullable JAVA_HOME/PATH/STARSECTOR_HOME and zero scoped surviving processes. Observer host JVM/compiler children retained 30-second/100000-byte bounds; actual JFR host recordings retained 8 MiB/five-second bounds. At execution all 3605 source pins, 81 artifacts and 9 installed-library pins passed. Later documentation-only edits are separately recorded; they do not reinterpret the older source cuts.

Primary raw evidence SHA-256: setup `951cb43620e965a94030e327b5b7cf2b7b4a85e13df30cca0dd6ecacdebfb59d`; quality `3ac1d5db4904bad9b8818769e599d10ffe7cc48529e734a1c39bc34e013ee578`; adapter `10e0bd5b745fa38ffb20be695c2722229eed9c19088c42247305118a6ccc0985`; combined host `f54ea645ac734dc2f1f53e1b97198e2015e86b1d30898293cce2f35642a2acf7`. Distinct per-phase postguard captures each hash `a6eef63a8c02239302b3e4e389abb246f57bf550abc40d9a83c0936d46a96835`; equal content does not mean a reused invocation.

The unchanged public workflow excludes `tests/game` from `test:tools`, and `syntheticCheck` excludes the licensed adapter. Exact-head hosted public checks/Java interop/release builds are still required before merge and do not replace these local licensed gates. The observer records only its own callback spans. Instrumentation overhead, frame percentiles, repeated late-game reference/control/shadow runs and economic-bridge p99 remain unmeasured. Issue #12 remains open; no authority/taint/save-durability certification is inferred.
