# Pure pacing checkpoint

Status: independent scoped R1 PASS; production and public tests remain frozen. This slice implements scheduling policy only. Issue36 still requires the separately reviewed miniature kernel, Store and guarded foreground composition. It adds no SQL, key, clock, entropy, network, native callback, CLI or authentication code. Root owns the no-dependency manifest/workspace wiring.

`Pacer::new(period_ms,now_ms)` accepts periods100..60000ms inclusive and checked initial deadline `now+period`. Observations are unsigned milliseconds from one caller-selected monotonic origin. `observe(now)` returns an immutable data-only DueHint at or after that deadline, then retains one Outstanding state. Repeated observations, including large gaps, cannot issue another hint. `pause(now)` discards outstanding scheduling and suppresses hints; `rearm(now)` schedules exactly `now+period`, including after explicit resume or durable commit, discarding every missed slot. Equal observations are valid. There is no strategic tick, catch-up loop, collection, allocation or floating point.

Backwards observation through any method permanently poisons the instance. Checked initial overflow rejects construction; rearm overflow permanently poisons an existing instance. Every subsequent operation on a poisoned instance returns Poisoned, including pause/rearm; explicit new construction is the only scheduling recovery. This is no authorization recovery. DueHint contains only original deadline and observed milliseconds, with private fields/read-only getters; old retained hints never certify activity, identity or durable success. Rearm itself verifies no commit.

Public behavior RED: a supported API stub returned no hints and accepted backwards observations. The native focused test run compiled successfully but all five tests failed on expected behavior (five-minute gap needed one hint; due boundary required a hint; backwards input required BackwardsObservation). These are behavior failures, not missing targets/import/tooling. After implementation the identical five groups passed, strengthened with all nine live phase/method backwards-observation combinations. The finite corpus includes40 period/gap pairs and explicit period/maximum-time boundaries. Tests cover long-gap one-hint suppression, reset from current time rather than prior deadline, pause/resume, equal observations, all entry points after permanent poisoning, explicit new-instance recovery, invalid period extremes and maximum valid deadline. Finite tests do not prove every possible sequence.

Observed commands on pinned Rust1.98.1:

- `cargo test -p nf-world-driver --test pacing --locked --offline`: RED5 failures, then GREEN5 passes.
- `cargo test -p nf-world-driver --locked --offline`:5 public tests PASS, no ignored tests.
- `cargo fmt -p nf-world-driver -- --check`: PASS.
- `cargo clippy -p nf-world-driver --all-targets --locked --offline -- -D warnings`: PASS.

Actual package-native harness: `npm exec -- workspace-template verify .tmp/pacing-review --scope module --module node:. --timeout 180000`. Private artifacts `.tmp/pacing-red.json`, `.tmp/pacing-green-focused.json`, `.tmp/pacing-green.json` record the actual result; final three structured child steps each report windows-job-object ownership, status0 and no timeout. No independent review verdict is inferred from author checks.

Future ordinary composition must derive observations from its actual std::Instant origin with checked conversion, invoke only guarded MiniatureStore auth/activity/commit APIs, and rearm after actual DurableAck or explicit pause/resume. Caller-supplied millisecond observations and DueHint must never satisfy a Store activity lease, authority proof or advancement check. Before any actual advancement the Store independently checks its own live Instant, current durable membership/authority, original signatures and purpose-bound one-use tickets at its final admissibility cut. This pure policy cannot certify that composition, freshness, durable restart, pacing hardware, native lifecycle or issue36 acceptance.

## Frozen source checkpoint

SHA256 below covers UTF8 source with CRLF normalized to LF, matching repository text policy. No semantic bytes depend on platform checkout line endings. The evidence document does not hash itself.

| Path | SHA256 (LF source) |
| --- | --- |
| crates/nf-world-driver/src/lib.rs | e6a2d144932869c859fc44af767eece459e29881ff38981832811b08bc4a5f23 |
| crates/nf-world-driver/src/pacing.rs | dacaad39fcf51377eb5f2cee6b4c974d61acac2d58358e4e33ef47c45b9f3de0 |
| crates/nf-world-driver/tests/pacing.rs | 9a1cb5687c2922285be394caf69849682f90c3126b4c6b391c1da6166eb81856 |

Independent coordinator R1 inspected all source/tests/docs and independently ran the same three checks through native Windows Job Object supervision (.tmp/pacing-root-native.json): five public groups, fmt and strict all-target Clippy PASS, each status0/no timeout. No IMPORTANT finding remains in this pure pacing slice; the LF source checkpoint matches. This supplies no actual Store/runtime authentication or complete issue36 acceptance.
