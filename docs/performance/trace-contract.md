# Trace fields, clocks and interpretations

One run uses one random 128-bit hexadecimal `traceId`; IPC operations can carry a separate 128-bit `requestId`. Do not derive either from account identity, paths, keys, chat or save names. Numeric `frameId` identifies an end-to-end frame inside that run. Thread identity includes process scope (`node` or `adapter`); adapter and JFR rows refer to the same JVM, while an equal numeric node thread ID stays separate. Select one authoritative frame capture source; duplicate identified frames across adapter/JFR reject, while unidentified frames cannot be deduplicated. Consumers must not count adapter and JFR copies twice.

Events are NDJSON objects with required `traceId`, `source` (`adapter`, `node`, `jfr`), `kind`, and nonnegative safe-integer `atUs` (UTC epoch microseconds). Supported fields are declared by `tools/performance/contract.mjs`; all other fields disappear. Durations are integer microseconds, bytes are byte counts, and CPU/network fields are cumulative counters. Counter restart requires a fresh trace ID.

| Kind | Numeric evidence | Meaning |
|---|---|---|
| `frame` | `durationUs`, recommended `frameId` | Actual whole-frame elapsed time from a verified frame seam; campaign `advance(float)` amount alone is not a measured frame duration. |
| `capture`, `encode`, `apply` | `durationUs`, `frameId`, optional `requestId` | Measured phases; budget groups synchronous capture+apply per frame. Encoding is reported separately. Incomplete/ungrouped groups stay unmeasured. |
| `thread_cpu` | `threadId`, cumulative `cpuUs` | Difference between at least two actual per-thread CPU-counter observations. One sample remains null. No inference from aggregate CPU usage. |
| `thread_running_sample` | `threadId` | JFR execution/native-method samples: counts, not exact running time. |
| `thread_blocked`, `thread_io`, `thread_sleep` | `threadId`, `durationUs` | Recorded monitor/park/wait, file/socket I/O, and sleep durations. These can overlap GC or each other; they are not a disjoint time partition. |
| `gc_pause`, `gc_cycle` | `durationUs` | Pause phases are distinct from the whole GC-cycle duration; do not treat an entire concurrent collection as a stop-the-world pause. |
| `allocation_sample` | weighted `bytes` | JFR allocation sampling estimate. The estimated rate is not an exact allocation counter. |
| `save`, `load` | `durationUs` | Timed supported lifecycle spans; callbacks alone do not certify durable save bytes. |
| `queue` | `queueItems`, `queueBytes` | Telemetry/adapter/node queue snapshots; peaks report observations, not a configured bound for an uninstrumented queue. |
| `resource` | optional `rssBytes`, cumulative `cpuUs`, `sentBytes`, `receivedBytes` | Process samples and counter deltas by source; at least two counter samples are needed. |
| `stale` | timestamp, optional `requestId` | Stale-result rejection count. |

Run metadata requires `schemaVersion: 1`, trace ID, exact `manifestSha256`, `budgetsSha256`, matching starting `checkpointSha256`, scenario, repeat, `warmupUs`, `durationUs`, speed, paused state, instrumentation (`none`, `frame`, `frame+jfr`), actual `campaignThreadId`, `startEpochUs`, `alignmentUncertaintyUs`, `droppedEvents`, `invalidEvents`, and baseline class (`synthetic` or `game`). `control` additionally requires `controlEvidenceSha256`; a different control manifest must declare `referenceManifestSha256`. Actual game runs additionally require `hardwareSha256` and `liveJvmFlagsSha256`, identifying locally retained hardware details and flags from the owned live process. Configured `vmparams` alone is insufficient. Those values identify evidence; they are not verified automatically against an uncontrolled process. A numeric ID is required rather than publishing a private thread name. The collector's final loss counters must populate metadata after recording stops.

`controlEvidenceSha256` identifies a stable control-method certificate, not a
per-recording artifact. Retain that certificate locally: it describes the
supported disabling seam, method/configuration and intended reference/control
manifest and live JVM flag identities. Every control repeat in a comparison
must supply the same certificate hash; inconsistent certificates refuse rather
than retaining only the first repeat's provenance. The comparison's certificate
applies to every listed `repeatId`. Per-run traces and artifacts keep their own
identities outside this field. Hash syntax and consistency do not attest the
certificate's contents or certify a live game experiment.
Existing reports that used per-recording hashes in this field must instead
supply the actual retained method certificate; the comparator does not invent
or normalize one from differing recording hashes.

Every scenario must retain one live JVM flag identity across its repetitions.
A named `control` or `instrumentation-off` scenario may deliberately differ from
the baseline; that allowance never permits B/C flags within the same scenario.
Synthetic input without live flags remains synthetic and cannot become recorded
game evidence through comparison.

The measurement window is `[startEpochUs + warmupUs, startEpochUs + warmupUs + durationUs)`.
One whole-phase rule governs both phase percentiles and capture/apply bridge
pairs: a span starts inside the window and ends at or before its end. A span
crossing either boundary is excluded, without clipping or adding its duration
to a complete bridge sample. Any overlapping capture/apply span crossing a
boundary makes that frame's bridge group incomplete, including when another
span of the same kind is wholly included. Entirely outside spans do not create
groups. `bridge.excludedSpans` counts overlapping capture/apply exclusions;
`quality.excludedPhaseEvents` counts overlapping excluded frame/latency phases.
Included counts remain visible in each percentile result. Wait/I/O duration
summaries retain their separate ending-boundary clipping semantics. Record
starts after warm-up and leaves a guard interval at the end so long operations
crossing boundaries cannot disappear from the benchmark unintentionally.

`clock.mjs` aligns monotonic nanoseconds to an explicit epoch anchor, with declared uncertainty. Its shell samples `Date.now` between monotonic samples outside frame work and records at least the clock's millisecond resolution. Compare start/end anchors: wall-clock jumps beyond uncertainty invalidate alignment. JFR provides its own event start instant/duration; align all same-host records to this microsecond timeline and preserve uncertainty. Do not assume peer machines' wall clocks are synchronized or use their absolute timestamps as one-way network latency. Correlate peer operations by request ID and measure local durations independently.

Nearest-rank p50/p95/p99/p99.9 describe observed samples. The analyzer warns below 10,000 frame samples for tail interpretation. Long frames exceed the configured `longFrameUs` threshold (initial 16,667 µs); this is a reporting threshold, not a promise of 60 FPS. Per-run p99 means and sample standard deviation summarize repetitions. Min/max overlap is descriptive and never a significance test or a speedup certificate.

The importer/exporter recognizes a closed set of metrics and does not preserve raw stacks. To attribute a cause, inspect the private JFR locally and correlate game-thread samples, lock owner/stacks, I/O, GC phases and frame tails. A monitor wait sample, a JFR allocation weight, or low aggregate utilization alone does not diagnose the late-game problem. Missing event kinds or unattested game-thread IDs leave that claim unresolved.
