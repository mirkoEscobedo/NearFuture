# Miniature World pure-core publication evidence

Implementation: ec33a40d271cafd0bf2fe674dec0ef297b4ba387, based on accepted main19dfc4e85f1538a1388277577ac78a65f691015a. This is phase1 of issue36; persistence, authority/activity policy, pacing, CLI and restart acceptance are not implemented here. Native game and transport acceptance are not inferred.

Independent read-only R1 found one Important defect: otherwise legal industry/arrival checkpoints with due changed to100 were accepted at committed tick1. One author repair introduced separate final and intermediate validation phases. Final state requires remaining horizon1..2/1..3; action-output uses actual committing N and additionally permits old due==N until exact timer reduction. Checked subtraction avoids maximum-tick overflow. Independent R2 PASS inspected all owned source/docs/fixtures and re-ran both original public-API probes; both now reject InvalidValue. Twenty public tests, strict all-target Clippy, formatting, the independent golden and all29 checkpoint hashes passed under native Windows Job Object supervision. No unresolved Important pure-core finding remains. This validates snapshot consistency, not creation/history provenance or operation authorization.

The isolated publication checkout contained accepted main plus this reviewed core/admission metadata. Unfinished transport and future kernel/storage integration were excluded. All44 staged text paths matched the tested checkout after Git newline normalization, and all29 checkpoint hashes matched staged Git bytes before the implementation commit. The lock adds only nf-world with existing nf-contract/SHA256 dependencies; all106 accepted package identities remain. Architecture admits the bounded no_std core with local contract and exact no-default-features sha2 only. Fixture generation is mandatory in check:schemas.

## Fresh publication gate

The package-owned native verifier ran six structured steps through windows-job-object; every step returned status0 and timedOut=false, overall PASS:

1. Offline npm ci --ignore-scripts in the isolated checkout.
2. Full npm run check: formatting, strict Clippy, TypeScript, architecture, independent generators,177 passing standard Rust tests,41 passing Node tests and actual synthetic Java tasks.
3. Offline Gradle exports for the real Java IPC and differential classpaths.
4. Explicit actual game-runtime Java17 IPC socket interop: mutual control/bulk authentication with the owned foreground Rust node,13.78s.
5. Explicit actual Java17 raw-input differential oracle: all26 cases,0.18s.
6. Locked offline workspace release build.

Harness command: npm.cmd exec -- workspace-template verify .tmp/miniature-core-full-review --scope module --module node:. --timeout 300000. The thin-state Node harness supplied explicit structured commands against .tmp/miniature-core-gate, inherited NF_IPC_JAVA/NF_IPC_JAVA_CLASSPATH_FILE and NF_SHADOW_JAVA/NF_SHADOW_CLASSPATH_FILE, and used exported real classpaths. Saved local result: .tmp/miniature-core-full-native.json. No shell imitation of a Windows Job Object was used.

Independent genesis golden:1018 bytes, SHA2561ed2a52abcd828aacce3ae7efa86239b04323f0a253ea3652b3784953d78f93f. NF-MINI-1 ruleset SHA256693eb02d8e5d64ba12678e8f5429a4760f06c0187225a53cf98c15c068b41595. Tests observe exact costs/conservation, deterministic first-legal conflict ordering, strict codec/counts/truncation/finite mutations, stale tick plans, one-use N+2/N+3 timers and same-tick action/timer replay. Finite tests do not establish all-input proofs.

The Java reference TSV policy now explicitly preserves LF checkout bytes. Existing recorded corpus identities refer to normalized Git blobs; no logical corpus rows or expected results changed.

PR58 is published. Initial hosted run37556864749 passed Linux, but Windows failed the mandatory World fixture generator because Git automatic checkout converted the hexadecimal text fixture final newline to CRLF. A narrow /crates/nf-world/fixtures/*.hex text eol=lf policy fixes checkout representation without changing any model/codec/golden byte or relaxing exact comparison. A fresh actual Windows git checkout-index tree passed all three World/Nex generator checks under native Windows Job Object supervision, status0/noTimeout, preserving the1018-byte golden and exact SHA256. [Corrected hosted run37557328959](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37557328959) passed Windows and Linux, including mandatory actual Java21 socket/differential checks, release and Linux SQLite sync-EIO recovery. The duplicate push run37557325945 also passed. [PR58](https://github.com/mirkoEscobedo/NearFuture/pull/58) merged at2026-10-07 01:36:58UTC asbc29e8cffefafa33e87e7ff4ab1aa5f3a1b3510e. Issue36 remains open until separately reviewed typed kernel, atomic SQLite owner, fresh authority/activity, failure recovery and pacing evidence passes.
