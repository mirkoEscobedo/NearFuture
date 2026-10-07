# Near Future delivery

Mode: Ticketed. Goal: complete the implementation backlog in https://github.com/mirkoEscobedo/NearFuture using agents where dependencies permit. Issues #1–#4 are design references; #5–#48 supply acceptance contracts.

## Current outcome

Current: #10 authenticated nonblocking local IPC. Its portable slice passed independent Rust/Java source review R1 and an isolated full lane/release plus actual Java17 interop under native Windows Job Object supervision; hosted Windows/Linux checks passed and PR #55 merged; actual native lifecycle acceptance remains unavailable. Foundation #5–#7 passed scoped independent review and hosted Windows/Linux checks, merged in PR #49. Kernel #8 passed read-only review R2, its isolated full lane and hosted Windows/Linux checks; PR #50 merged and #8 closed. #9 persistence and #22 identity passed independent review and hosted Windows/Linux gates, merged in PR #51. #11 bounded capture/projection passed portable source review R2, its isolated full lane and hosted Windows/Linux CI; PR #53 merged. #13 Nex semantic boundaries passed portable source review R2, its isolated full lane and hosted Windows/Linux CI; PR #52 merged. #14 isolated Java reference passed portable source review R1, fresh checks on runtime21/17, an isolated supervised full lane/release, and hosted Windows/Linux CI; PR #54 merged. #15 pure Rust differential shadow passed independent review R2, its isolated full lane (142 Rust/40 Node), explicit Java17 socket/differential tests, locked release build and hosted Windows/Linux checks; PR #56 merged. A separate complete-world commitment passed independent review R1 and focused native-supervised checks; its isolated full lane (157 Rust/40 Node), explicit Java17 socket/differential tests, release and hosted Windows/Linux checks passed; PR #57 merged. #23 actual two-peer transport and #36 miniature World pure core are progressing in parallel; typed kernel/storage integration remains a separate review step. None has native game acceptance yet. #12 profiling tooling is reviewed as partial delivery; actual game measurements remain unmeasured. See docs/integration.md for the reviewed foundation revision.

Checks: deterministic runtime manifests and actionable failure diagnostics, source/license inventory, public Rust/Java tests, formatting/linting, dependency boundaries, missing-game-library failures, independent review. Game claims need exact-runtime game evidence.

## Ordered outcomes

1. Foundation (#5–#6), contracts/kernel/storage/IPC/snapshots (#7–#11). Profiling (#12) can proceed independently once builds and a valid scenario exist.
2. Local Nex boundary/reference/shadow/ownership/performance (#13–#17), then domain ports (#18–#21). Identity/transport/Chat (#22–#25) follows its own prerequisites in parallel.
3. Audited Market (#26–#31) and read-only Expanse (#32–#35), with demonstrated native recovery and lifecycle gates.
4. Canonical World (#36–#38), replicas/fenced handover (#39–#40), useful bounded workers (#42). Optional Raft (#41) retains its decision gate.
5. Migrations, packaging, operator tools, integrated reliability (#43, #45–#47). Optional Wasm (#44) and sharding analysis (#48) retain their conditional scope.

## Authority and stop conditions

Use locally supplied game libraries without redistributing proprietary binaries/assets. Keep private saves, secrets and identifying paths out of public evidence. Preserve offline workspace-template tooling. No performance claims without end-to-end measurement, duplicate strategic writers, automatic two-peer timeout failover, arbitrary Java-mod hosting, or divergent-history merging.

Accept issues only after fresh checks and independent review. Missing game checks remain unavailable; mocks do not satisfy native gates. Keep incomplete issues open with concrete gaps. Publish implementation through pull requests; do not post messages/comments without explicit authorization. Material infeasibility returns to planning without weakening acceptance contracts.
