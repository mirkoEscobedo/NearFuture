# Performance tooling and evidence status

NF-008 / issue #12 delivers trace tooling under its unavailable-reference-data contract. **The actual late-game baseline remains unmeasured, its cause is unknown, and no acceleration is claimed.** There is no installed Near Future frame hook or measured baseline/control/shadow campaign. The ≤2 ms bridge p99 value in [resource-budgets.json](../../config/resource-budgets.json) is an initial validation budget, not a measured achievement or FPS guarantee.

The implemented public seams are a bounded `TraceBuffer`, allowlisted event/run contracts, monotonic-clock alignment helpers, offline JFR export, percentile/resource analysis, matched repeated comparisons and a synthetic saturation CLI. All filesystem, process and wall-clock effects stay outside the calculation/queue core. No tool launches/attaches to Starsector or rewrites its launcher/mod/save files.

## Opt-in campaign callback observer

The local adapter includes a disabled-by-default observer of its own campaign callback. In a separately qualified adapter launch, opt in with both JVM properties:

```text
-Dnf.profiling.callback.enabled=true
-Dnf.profiling.traceId=<32 lowercase hexadecimal characters>
```

Use a fresh anonymous trace ID for each recording run and the same ID in its run metadata. The enabled value must be exactly `true`; a missing/invalid trace ID or absent sector attaches no observer. The observer starts no recording: an explicitly owned JFR session must enable `nf.CampaignCallback`. It records only the observer's own callback span and correlation ID, with stacks disabled. These spans do not measure a whole frame, other mods, the economic bridge or instrumentation overhead.

Repeated loads replace only the owned transient script. Before saving, the plugin closes and removes it; after a successful or failed save, it re-evaluates opt-in and attaches a fresh observer. Closed stale scripts emit no callback events, and unrelated scripts remain untouched. These lifecycle statements are tested on licensed API host proxies; actual campaign scheduling and save serialization remain unverified.

Local host checks compile classes for Java 17 using pinned JDK 21.0.8. The installed game's Azul 17.0.10 runtime has not exercised this observer. Required delivery evidence separately includes the licensed `gameAdapter` build, all six host contracts, and exact-revision public CI; synthetic CI excludes the licensed adapter build and `tests/game` contracts. The actual game baseline and bridge target remain unmeasured. See [verification evidence](verification.md) for commands, exact revisions and limits.

```powershell
node --test tests/performance/*.test.mjs
npm run check:types
node tools/performance/cli.mjs saturate --offers 1000000 --output .tmp/saturation.json
```

[Recorded synthetic saturation](synthetic-saturation.json) offered 1,000,000 events: 4,096 retained, 995,904 dropped, 449,450 serialized payload bytes queued, then zero items/bytes after bounded draining. This demonstrates item/byte/event bounds for telemetry. It does not establish game timing, total process RSS bounds, native adapter performance or IPC saturation. Consumer file writes happen after draining; the producer's `offer` does no file/socket I/O and returns false immediately on saturation/invalid input. This Node implementation must not be described as an installed Java game-thread logger. Control commands and economic receipts must never use this lossy buffer.

The CLI analyzes already captured private run files:

```powershell
node tools/performance/cli.mjs analyze --metadata .tmp/run.json --events .tmp/events.ndjson --manifest .tmp/runtime-manifest.json --output .tmp/report.json
node tools/performance/cli.mjs compare .tmp/baseline-1.json .tmp/baseline-2.json .tmp/baseline-3.json .tmp/shadow-1.json .tmp/shadow-2.json .tmp/shadow-3.json --output .tmp/comparison.json
```

Analysis verifies metadata against the exact supplied manifest and budget file SHA-256. Inputs are capped at 64 MiB, 100,000 records and the configured event byte limit; malformed/nonfinite events and cross-run IDs reject. Unknown fields are dropped before output. Missing observations stay null/unmeasured rather than becoming zero-cost claims. Known dropped/invalid telemetry blocks comparisons. Repeated reports require matching checkpoint, warm-up/duration, speed/pause, budgets and scenario instrumentation; unsupported config differences reject. A documented control can supply a distinct manifest only with its reference-manifest hash and supported-seam evidence hash. Those hashes identify the operator's evidence; this tool does not certify that evidence or mod compatibility.

JFR export reads an existing recording through the public Java 17 `RecordingFile` API, streams events, and emits only hexadecimal correlation IDs plus numeric timestamp/duration/thread/frame/allocation evidence. It strips thread names, stacks, file/socket names, arbitrary strings, environment/system properties, paths and private arguments. It rejects recordings above 64 MiB, scans above 1,000,000 events, or exports above 100,000 records. Discard partial output after a nonzero result.

```powershell
javac --release 17 -Xlint:all -Werror -d .tmp/performance-java tools/performance/jfr/ExportRecording.java
# Use an existing private recording from an explicitly owned JVM session.
java -cp .tmp/performance-java ExportRecording .tmp/private-recording.jfr <32-hex-trace-id> > .tmp/events.ndjson
```

The exporter also recognizes custom `nf.Frame`, `nf.Capture`, `nf.Encode`, `nf.Apply`, `nf.Save`, `nf.Load` and `nf.ThreadCpu` events. The bounded synthetic capture fixture proves those public JFR seams without game binaries. It is not a production frame hook. Compiler output and synthetic JFR capture/export were exercised on bundled Azul Java 17.0.10 outside Starsector; game script-classloader permissions remain unverified.

See [trace contract](trace-contract.md), [paired scenario methodology](scenarios.md), and [verification evidence](verification.md). Raw JFR files and save data stay private under ignored local directories; review sanitized JSON before sharing it. No private recording is checked in.
