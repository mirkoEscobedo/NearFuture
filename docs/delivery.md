# Near Future delivery

Mode: Ticketed. Goal: complete the implementation backlog in https://github.com/mirkoEscobedo/NearFuture using agents where dependencies permit. Issues #1–#4 are design references; #5–#48 supply acceptance contracts.

## Current outcome

Current: #7 typed contracts and Java/Rust interoperability, including independent review and one bounded semantic repair. Foundation #5–#6 passed scoped local review; hosted Windows/Linux evidence awaits the implementation PR. #12 profiling tooling is reviewed as partial delivery; actual game measurements remain unmeasured. The dependent #8 world kernel is being prepared in parallel and is not accepted yet.

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
