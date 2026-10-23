# Headless CI workload and qualification

## Complete Rust partition (#98 recovery)

The required `public-rust` group now runs three suites on each of Linux and
Windows, retaining the existing 20-minute job bounds and `fail-fast: false`.
The selector reads locked/offline Cargo workspace metadata and validates the
actual workflow matrix before executing Cargo. Every workspace package belongs
to exactly one suite; new packages enter the remainder. Missing/duplicate suite,
platform or required package selections refuse execution.

| Suite | Complete package selection |
| --- | --- |
| store | nf-store |
| peers | nf-identity, nf-ipc, nf-transport |
| remainder | All other actual workspace members, excluding those four packages |

Each command uses ordinary `cargo test` selection, including unit, binary,
integration and doc tests, locked/offline dependencies and the retained #98
`--test-threads=1` invocation. No target/name filter, new ignore or production
deadline change is introduced. Store retains its fresh caller-owned restore
refusal inputs and original artifact retention. Other operator, interop, release,
profile and Linux fault checks remain required. Local `npm run check` still runs
the full original aggregate.

The same six required groups and strict always-running qualification check
remain. Six Rust matrix children replace two: fifteen total required check
records must succeed. The workflow validator rejects missing cells; GitHub's
matrix result and the existing qualification CLI refuse failed, cancelled,
skipped or absent required groups. Inspect actual child results before landing.

The retained #98 PR logs show successful progress until the overall 20-minute
cutoff, including completed `driver_cli_actions` in another attempt and the
same-head push. They do not establish a driver deadlock. Historical failed
attempts and repair/retry counts remain recorded separately from the human's
bounded recovery authorization. Partitioning addresses accumulated workload;
it does not explain unrelated ACL/private-storage failures or guarantee timing.
Current target/head checks and protected serial landing still govern acceptance.

## Earlier build/Rust separation (retained evidence)

Refs #92, #93. This is a capacity repair within the protected headless integration profile. It preserves the complete check workload and all child job deadlines. The original blocked candidate `13d5efa` and its failed runs remain evidence; this change neither explains nor repairs opaque private-storage or Linux fault-worker failures.

The Windows public jobs on [the #95 candidate](https://github.com/mirkoEscobedo/NearFuture/actions/runs/39308549553/job/118251855188) and [the #96 push](https://github.com/mirkoEscobedo/NearFuture/actions/runs/39312371487/job/118264978891) exceeded the existing 20-minute limit. In the latter, workspace Rust completed around 19:53:36 UTC, tooling began at19:53:43, and cancellation occurred at19:56:24; the job began at19:36:24. This supports giving the full Rust suite and the remaining checks their own bounded jobs. A cancelled game-lane subprocess is not independently established as a product defect.

| Required group | Preserved commands |
| --- | --- |
| public-rust, Windows/Linux | Exact `npm run test:rust`, which expands to the complete locked/offline workspace Cargo suite. Same checkout/Node/Java/Rust pins, locked fetch and offline vendored installation as the original public job. |
| public-build, Windows/Linux | `npm run lint`, `npm run check:schemas`, `npm run test:tools`, `npm run check:java`. The existing Linux Java IPC, real reference differential and both sync-fault checks remain in this group with their original arguments/environments. |
| windows-interop | Existing fresh Java prerequisite, actual Java/Rust IPC and reference differential commands unchanged. |
| release-build, Windows/Linux | Existing locked/offline workspace release build unchanged. |
| profile-smoke | Existing profile regressions and accepted executable smoke unchanged. |

The split only removes Rust test execution from public-build and executes it in a new required matrix. Existing public-build, Windows interop, release and profile job names and deadlines remain. Both new Rust children retain20 minutes; matrix fail-fast remains false. The local `npm run check` still executes the complete original aggregate, and no package script is narrowed.

The manifest lists public-rust explicitly, and the always-running headless-qualification job depends on it alongside every existing group. Matrix success requires both platforms to pass. Public CLI regression tests independently require the new group and refuse its missing, failed, cancelled, skipped, queued or absent result. The existing required check context and no-bypass branch protections stay intact; prior retry and semantic repair budgets are not reset. A new candidate must pass every current required group before an ordinary expected-head PR merge.

Enabled capability remains build identity only, persistent writes off. This CI repair grants no native game, writer, peer runtime or managed-upgrade qualification.
