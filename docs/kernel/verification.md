# NF-004 verification checkpoint

Source checkpoint: owned `crates/nf-kernel/**` and `docs/kernel/**`, 2026-10-07. Git revision is supplied by the coordinator after review; these records do not claim an uncommitted SHA as an implementation revision. This is synthetic kernel evidence, not actual-game compatibility, offload speedup or a Nex certificate.

Meaningful vertical RED/GREEN cycles at public seams:

- Invalid relation references: missing World API → World::new returns InvalidReference.
- Scoped RNG: missing API → fixed independent Node SHA-256 fixture 15213394994864267995; term/session are absent from the scope type.
- Versioned snapshot: missing codec APIs → exact domain7/type1/schema1 header and restore; reordered constructor vectors failed equality → constructor normalization.
- Exact missing jobs: missing scheduling API → Pause returns exact missing IDs without mutation; complete Recompute initially returned InvalidFrontier → independent peace/market commit, score -495 and credits107 from external expectations.
- Conflict: both overlapping jobs applied, score -486 → stable first winner, score -495, persisted Conflict loser independent of reverse arrival.
- Unauthorized writes and stale proposal revisions initially returned InvalidProposal → specific persisted Unauthorized and StaleRevision reasons.
- Draw overflow initially collapsed to ProviderFailed → deterministic Overflow retained.
- Persistence: missing frontier/batch/replay APIs → snapshot + exact frontier + committed batch restore with event-only replay.
- Extra replay event with matching after-state hash initially applied → reject InvalidProposal outside exact admitted frontier.
- Duplicate durable snapshot outcomes initially restored → DuplicateIdentity before activation.

Additional adversarial regressions cover target/module rollback when a revision overflows, provider failure, unauthorized/stale intents, changed leader/session fencing without reroll, unknown/duplicate result IDs, every truncated snapshot prefix, excessive counts, unsorted IDs, unregistered domain7, input/batch hash mismatch and the 1 MiB input envelope.

The actual process-restart harness is `cargo test -p nf-kernel --test restart --locked --offline -- --nocapture`. It launches nine sequential foreground children of its own test executable, each with a 15-second timeout/kill/reap path. It creates private synthetic temporary fixtures, exits after five ticks with a provider snapshot and exact pending frontier, resumes in another process with changed authority/session, and replays all ten committed batches in a third process without provider execution. Worker counts 1, 2 and 4 use actual scoped Rust threads; even counts reverse returned results. Final snapshot bytes match across workers/restarts/replay. Independently computed final relation score is -443, credits170, draws10, cooldown tick10, outcomes20. State SHA-256 is `bf38b403751b268e80c0d3998acfac60adfab56651c717cc748926c304104d9c`.

Focused commands executed successfully during implementation: `cargo test -p nf-kernel --locked --offline`; `cargo clippy -p nf-kernel --all-targets --locked --offline -- -D warnings`; `cargo fmt -p nf-kernel -- --check`. Final fresh counts/results are appended after all changes. No test is ignored. `scenario_child` is the process-harness entry point and returns immediately when invoked without its parent-owned synthetic role.

`workspace-template verify --scope module --module rust:crates/nf-kernel --timeout 60000` returned INSUFFICIENT_EVIDENCE with no steps: the pre-existing root Cargo/npm package name/path collision prevents module graph execution. Ordinary focused commands remain available. Root verification has a Windows Job Object provider; the coordinator owns final composed repository evidence and independent review.

Residual gates: no Java domain7 vectors/capability registration, storage durability, authenticated leadership, production provider migrations, ownership handover, general scheduling framework, retention compaction, economic reservations/native transfer, game launch or game mutation. Current static proposal validation recomputes its small calculation; no benefit is claimed. Review and integration are required before issue closure.

Final fresh checkpoint: all 21 kernel integration tests passed (including the child-process harness), fmt check passed, and all-target clippy with warnings denied passed. Largest production file is jobs.rs at298 lines; largest test is restart.rs at229 lines, below packaged architecture warning thresholds. Read-only source inspection found no std/game/network/database/unsafe APIs in the production kernel and no private paths/credentials in owned source/docs. Coordinator integration and independent review remain pending.

Independent review round1 found an important quota timing gap: per-list checks bounded the work, but combined snapshot entities and nested frontier entry totals were rejected after construction. Repair1 threads one EntryBudget through nested snapshot/intent decoders and encoders; collection counts reserve the cumulative quota before iterating, byte-field lengths use a separate path, and relation/market counts are limited by the remaining combined entity capacity. No unbounded-allocation claim was made.

The public regression initially returned InvalidValue for all three excess declarations (a truncated 129th entity, a truncated quota-crossing nested intent, and a full 16,768-entry frontier with a malformed final command). After repair all return Limit before reading the absent/malformed later tuples. The independent raw-byte fixture has a188577-byte snapshot and444970-byte outer frontier; a16285-entry frontier and the same large snapshot roundtrip successfully, confirming byte lengths are not counted as entries. Fresh repair checks: fmt check, all-target clippy -D warnings and all22 kernel tests passed; restart/worker-count snapshots retain the same recorded hash. Coordinator read-only repair re-review R2 passed: source inspection and fresh fmt, all-target Clippy and all22 tests passed. Root-owned architecture lint changes also passed a separate read-only R1 review. No unresolved important finding remains in this kernel scope. Hosted Windows/Linux execution is recorded by integration after publication.
