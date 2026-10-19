# Integration and profile policy

Refs #92, #93, #6, #45, #47. This is a Ticketed integration track. The live
issue contracts and their commit/check evidence control selection and completion.
Bootstrap #93 establishes protected serial dev integration and the named headless
profile. The live #92 status identifies the current writer and actual landing
blockers; prepared, pushed and landed are distinct states. Outcomes follow #92's
ascending execution ranks, explicit technical
dependencies, landing dependencies and conflict keys. One writer and integrator
own the current leaf. An issue being closed is not dependency evidence by itself.

## Branches and review

`main` remains the default release line. `dev` starts at fetched main
`10b4617f93ee12b196a2b055566153b72330d108`. Ordinary `codex/<issue>-<topic>`
branches start from current dev and PR to dev; declared stacks may initially
target their predecessor. No direct writes, force pushes, branch deletion or
protection bypass are part of landing. The main/dev/release ruleset requires
`headless-qualification`, a current base, a PR and resolved conversations.
There is no merge queue; one integrator lands candidates serially.

Record independent read-only review against the exact candidate diff, including
spec/authority, code/tests and operations/security lenses. The current sole
collaborator cannot approve their own PR. Required GitHub approvals are therefore
zero; independent agent review is recorded as such, never as human approval.
If a distinct authorized reviewer joins, reconsider required approving identities.
Ruleset readback and actual refusal evidence, rather than this document, establish
enforcement. An unavailable administrative operation blocks #93 acceptance.

Before each merge, fetch target/head, record both SHAs, require the reviewed head
and current target, and inspect every required job. Cancelled, skipped, missing
or failed required work refuses qualification. Use the expected head SHA in the
merge request and recheck the target immediately before it; GitHub's strict base
requirement closes the race. Use a merge commit. Never use an admin override.
Record integration SHA, jobs, enabled scope and limits before closing the leaf.
Use `Refs` for partial parent work. Keep broader parents and unique historical
PR work until their own complete acceptance is established or preserved.

## Qualified scope

`config/profiles/headless-foundation.json` names `headless-foundation-v1`.
Its runnable node only reports synthetic build identity; the existing CLI tests
verify exact output and refusal of unsupported arguments. `npm run check:profile`
executes that smoke and the profile regressions. `npm run check` preserves the
existing Rust, Java, schema, tooling and architecture checks. Hosted qualification
also requires Windows/Linux release builds, both platforms' Rust/Java IPC and
reference differential checks, and both existing required Linux SQLite sync-fault
cases. Native licensed checks are explicitly optional for this headless profile.
They are required when a later profile claims the affected native behavior.

The always-running aggregate gate accounts for every registered required job;
matrix success means all children succeeded. Every newly accepted usable path
must enter the named profile and its required regression gate before landing.
Do not describe synthetic tests as a game release or offload certification.
Game lifecycle, local shadow, native writer, peer runtime, Chat runtime and
managed upgrades remain disabled and absent from the accepted entry point.
Library/test binaries are development surfaces, not activated profile services.

## Data and recovery

Development, test and release use separate roots declared in the profile.
All persistent writers are disabled in the initial profile: the accepted root
executable opens no database and starts no service. `admitWrite` supplies a pure
schema/provider/writer and environment preflight for later profile composition;
it refuses this profile before filesystem effects. It does not replace a future
writer's native lock, authorization, canonical filesystem containment or migration
transaction. No existing library writer is certified by this helper. Enabling a
writer requires wiring this admission at its real open boundary and proving those
additional controls. No migration or existing data root is opened by this change.

Before any future writable profile, inventory existing roots and persisted format,
provider and writer identity; verify compatibility before opening for writes.
Document abort, downgrade and recovery for each persistent transition. Reverting
source or disabling a flag cannot reverse database migration, retirement,
protected-frontier advancement or game saves. Refuse incompatible roots; do not
automatically migrate release data during a development/test launch.

## Promotion and hotfixes

Promote a qualified dev through a separate reviewed PR to main. Use a short-lived
`release/<candidate>` only for a stabilization freeze. Preserve ancestry with
merge commits between long-lived lines; bring main/release fixes back to dev.
Hotfixes start at main and return to both lines. A promotion records candidate
and integration SHAs, artifact SHA-256, profile/config SHA-256, schema/provider/
writer compatibility and qualification limits. Initial schema/provider/writer
values are null because no writer is enabled. Release publication, production
installation, live-save writes and legacy branch deletion are outside this task.

## Local preservation and verification

The original checkout remains on its existing receipt-notification branch. Seven
modified tracked paths, untracked work and eight other worktrees were inventoried
before creating the isolated #93 worktree. Private path/hash inventory and current
execution logs are under ignored `.tmp/integration-coordinator/`. Do not stage that
directory, machine paths, saves, licensed inputs or the mixed original checkout.
Recheck the inventory after work. Existing worktrees are not disposable candidates.

The package's root auto-detection runs only root Cargo tests in this polyglot
repository. The thin project override names one `foundation` aggregate module
so Windows Job-owned verification runs the actual npm gate, profile smoke and
locked/offline workspace release build. On Windows:

```powershell
npm ci --offline --ignore-scripts --no-audit --no-fund
npm.cmd exec -- workspace-template verify . --scope module --module foundation --timeout 1200000
```

The Windows override uses the fixed `cmd.exe /d /c npm.cmd` entry point because
the native process runner cannot launch npm's command shim as a native executable.
Linux/hosted checks continue to run the repository's portable commands directly.

## Diagnostic/tooling preparation (#94)

The source deltas are preserved as separate commits, with only the tooling-test
registration conflict resolved by retaining both profile and parser globs.
The original source PRs remain open until a qualified successor actually lands.

| Source | Pinned source head | Current disposition |
| --- | --- | --- |
| #86 | `20a757b2a64dd418b20882e81da1539ebcd9849b` | Prepared: test-only ACL fixture serialization; production deadline unchanged |
| #89 | `b5195a09d477525193c35b10365cfdb1f2405ea0` | Prepared: closed typed diagnostics, including single-call Java and world-driver fixtures |
| #88 | `8854e5140aca5f0b67f91944ac32675757836e1a` | Prepared: CLI2 observation parser and defensive refusal; no desktop runner or native qualification |
| #73 | `e5c9829005a0592454e0dba72152351d2e0b7b3d` | Prepared: historical scope/profiling notes only; no new session authority |

Diagnostics expose closed stages/reasons and numeric I/O/helper/cleanup outcomes,
with no paths, private identities, payloads, helper text or command arguments.
Legacy errors and one execution of the original create operation are retained.
The profile registers diagnostic and parser regression paths; its runtime-enabled
scope remains build identity only. Passing checks do not explain historical
PrivateStorage failures or certify native image capture. Refs #94, #46.

Independent review found that #88 admitted a PNG truncated immediately after its
dimensions. The first #94 semantic repair adds complete chunk framing, CRC and
bounded zlib/scanline admission, including Adam7 and indexed-image layouts.
Limits are 64 MiB each for the original PNG and inflated scanlines, and 16,384
chunks; larger attachments are refused. Valid accepted attachments retain their
exact original bytes and provenance. This checks image-stream integrity, with
the [PNG specification](https://www.w3.org/TR/png-3/) as its format reference;
it does not certify rendering, ancillary metadata semantics, native provenance
or freshness. The previous synthetic pixel had an invalid IDAT CRC and is
replaced by a valid encoded pixel without relaxing byte/provenance assertions.
