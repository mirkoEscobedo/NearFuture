# Headless CI workload and qualification

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
