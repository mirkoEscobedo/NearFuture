# Reviewed foundation integration

Implementation revision: [3bccb851c86efb953cd2b797f3382fb68256833d](https://github.com/mirkoEscobedo/NearFuture/commit/3bccb851c86efb953cd2b797f3382fb68256833d). Work date: 2026-10-07.

| Issue | Reviewed outcome |
| --- | --- |
| #5 / NF-001 | Runtime/source manifest and feasibility inventory accepted. Actual game capability gates remain explicitly unknown/blocked, as required by the spike's failure contract. |
| #6 / NF-002 | Build foundation accepted locally, including a clean isolated candidate checkout; [hosted Windows/Linux execution](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37543377882) passed; PR #49 merged and issues #5–#7 closed. |
| #7 / NF-003 | Typed canonical/wire/signature contract accepted after independent review round 2. This is a data/primitive contract, not session authorization or durable operations. |
| #12 / NF-008 | Profiling tooling accepted as partial delivery. Actual late-game baseline/control/shadow runs and instrumentation overhead remain unmeasured; keep the issue open. |

The complete public lane ran from an isolated checkout of the reviewed candidate, with no installed npm packages or Java output copied into it: `npm ci --offline --ignore-scripts --no-audit --no-fund`, `npm run check`, and `cargo build --workspace --locked --offline --release` all passed. The check ran 39 Rust tests, 36 Node tests, all Java executable assertion suites, strict Clippy/types/boundaries, formatting and all three fixture/dataset reproducibility checks. It used the locked open-source dependency cache. Missing-game-library tests exercised real Gradle failure diagnostics. The independent Java review also reran the complete synthetic suite on the installed Azul Java 17.0.10 JVM and compiled the adapter against the local game API; no Starsector campaign was launched.

The first clean-checkout run exposed automatic Windows CRLF conversion changing exact fixture bytes. Checkout attributes now force LF for canonical JSON/generated sources and preserve checksum-pinned Unicode source/license bytes. The full lane passed after that portability repair. The isolated checkout contains the foundation workspace only; concurrent kernel, storage and identity work is excluded from this revision and acceptance.

Review provenance: the coordinator independently inspected NF-001 runtime code and reproduced the observed manifest hash; a separate agent reviewed NF-002 builds; the protocol reviewer inspected both authored Rust canonical and Java implementations in a read-only round 2; the coordinator independently inspected the reviewer's Rust wire implementation. The coordinator reviewed NF-008 tooling and its bounded-read repair. Reviewers did not repair the code they reviewed. First-review findings were fixed with public regressions; no unresolved important finding remains in these accepted scopes.

The staged prohibited-content scan found no game JAR/assets, save files, private captures, keys or machine-specific installation paths. The only tracked JAR is the public Gradle wrapper. Open-source build packages and Unicode license notices retain their provenance. Runtime evidence contains relative artifact names, hashes and selected safe metadata rather than private launcher contents.

Residual limitations are retained in the component evidence: no game campaign/save/load/thread or writer-ownership certificate; no account authorization, transport, durable deduplication, cancellation handler or bulk assembler; finite mutation campaigns do not prove all possible inputs; logical decoder/queue budgets do not guarantee whole-process RSS. Both hosted Windows and Linux jobs passed the complete public lane and release build in workflow run 37543377882. Kernel #8, persistence #9 and identity #22 are separate in-progress outcomes, not silently accepted by this foundation.

## Reviewed kernel checkpoint

Implementation revision: [b9d2f6470a4a1585874aae2b683b7e966b665e4b](https://github.com/mirkoEscobedo/NearFuture/commit/b9d2f6470a4a1585874aae2b683b7e966b665e4b). Issue #8 passed coordinator read-only review round2 after one quota repair; the author's separate review accepted the coordinator's architecture boundary changes. No unresolved important finding remains in this scope.

A fresh isolated checkout containing the foundation plus kernel passed offline npm installation, `npm run check` and the locked offline workspace release build. It ran61 Rust tests,38 Node tests and the Java assertion suites. The kernel's22 tests include nine actual child processes across1/2/4 threaded workers, restart with changed leader/session and event-only replay; every final state hash was `bf38b403751b268e80c0d3998acfac60adfab56651c717cc748926c304104d9c`. Independent malformed prefixes verify cumulative16384-entry and combined128-entity quotas reject before expanding the exceeding collection. Component details are in [kernel/verification.md](kernel/verification.md).

Both hosted Windows/Linux jobs passed [workflow37544651825](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37544651825); [PR #50](https://github.com/mirkoEscobedo/NearFuture/pull/50) merged and issue #8 closed. Persistence, identity, Java domain7 parity and wire capability registration are excluded from kernel acceptance; actual game gates remain unmeasured. Storage/identity working files were excluded from the isolated checkout and commit.
