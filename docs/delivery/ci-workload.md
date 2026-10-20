# Headless CI workload and qualification

Refs #92, #93, #102. The protected headless profile composes the #93 capacity repair with the receipt vehicle's serial identity and transport partitions. It preserves the complete check workload and all child job deadlines. The original blocked candidate `13d5efa` and its failed runs remain evidence; this composition neither explains nor repairs opaque private-storage or Linux fault-worker failures.

The Windows public jobs on [the #95 candidate](https://github.com/mirkoEscobedo/NearFuture/actions/runs/39308549553/job/118251855188) and [the #96 push](https://github.com/mirkoEscobedo/NearFuture/actions/runs/39312371487/job/118264978891) exceeded the existing 20-minute limit. In the latter, workspace Rust completed around 19:53:36 UTC, tooling began at19:53:43, and cancellation occurred at19:56:24; the job began at19:36:24. This supports giving the full Rust suite and the remaining checks their own bounded jobs. A cancelled game-lane subprocess is not independently established as a product defect.

| Required group | Preserved commands |
| --- | --- |
| public-rust, Windows/Linux | `npm run test:rust:ci-public`: locked/offline workspace Cargo tests excluding only `nf-identity` and `nf-transport`, which have complete required partitions below. Same checkout/Node/Java/Rust pins, locked fetch and offline vendored installation. |
| critical-io-contracts, Windows/Linux | Complete `nf-identity` package tests with `--test-threads=1`, including unit, integration and documentation tests. |
| critical-transport-contracts, Windows/Linux | Five serial package partitions: library `notification_effects::`; library `--skip notification_effects::`; all integration tests (`--test '*'`); all binaries (`--bins`); all documentation tests (`--doc`). The complementary library filters cover the complete library suite. |
| public-build, Windows/Linux | `npm run lint`, `npm run check:schemas`, `npm run test:tools`, `npm run check:java`. The existing Linux Java IPC, real reference differential and both sync-fault checks remain in this group with their original arguments/environments. |
| windows-interop | Existing fresh Java prerequisite, actual Java/Rust IPC and reference differential commands unchanged. |
| release-build, Windows/Linux | Existing locked/offline workspace release build unchanged. |
| profile-smoke | Existing profile regressions and accepted executable smoke unchanged. |

Every child retains its original 20-minute deadline and matrix fail-fast remains false. No package, assertion, process check or documentation test is removed. The local `npm run check` still executes the complete aggregate. Its `test:rust` command runs the locked/offline workspace excluding transport, then the complete transport package serially; this retains the older #65 scheduling choice without copying its older implementation.

The manifest and always-running `headless-qualification` gate require all seven groups above: 20 child results and the aggregate, 21 required job records. Matrix success requires every platform and partition to pass. Public qualification regression tests refuse missing, failed, cancelled, skipped, queued or absent required group results. The existing required check context and branch protections stay intact; prior retry and semantic repair budgets are not reset. A new candidate must pass every current required group before an ordinary expected-head PR merge.

The inherited `private-acl-phase-observation` job runs only on pushes to `codex/receipt-main`. Its skip on an integration PR or dev is expected: it is optional, excluded from qualification and receives no required-check credit.

The profile registers the receipt/Portal library regression path through the required transport partitions. Enabled capability remains build identity only, persistent writes off. This composition grants no native game, writer, peer runtime or managed-upgrade qualification. See [receipt convergence and preserved scope](receipt-convergence.md).
