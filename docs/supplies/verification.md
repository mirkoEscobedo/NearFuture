# Supplies verification

Implementation revision: [`6055558fa1f00fbe20a03871ee302615aec3a373`](https://github.com/mirkoEscobedo/NearFuture/commit/6055558fa1f00fbe20a03871ee302615aec3a373). Documentation is a separate update; all 47 implementation paths remain unchanged. Base: `64037d5b174b61e3bfc0ba0a6ac7f22f34fff5cf`, tree `807f00e804cd93560a5fb50f2855c312df91c65a`. Branch: `codex/supplies-vault`. No issue closure or hosted acceptance is claimed.

The original `05274d7` local proof cut has 45 supplies additions and 2 additive module mounts, with all 800 other Main checkout files and 957 user-workspace files preserved. Existing Git checkout line-ending conversions were retained. Rust/supplies code is unchanged by these documentation additions. The 47 changed source paths have SHA256 `16938fd680cb129d1e7be756154e7b45255a550bb2ccafb87dad0a4786726ae0` over sorted `path + space + file-SHA256 + LF` records.

On 2026-10-08 the formatted Main-based cut passed 42 distinct supplies behavior cases: kernel codec 4, store group 1 fifteen, store group 2 nineteen, and conservation 4. The ignored management-only crash dispatcher is invoked by the two owned crash tests and is not a behavior case. An initial library filter selected zero kernel tests; that observation is retained but supplies no acceptance. The corrected kernel integration target passed all 4 actual cases.

Format check, full workspace/all-target Clippy with `-D warnings`, TypeScript, architectural boundaries, schema reproducibility and all 85 tooling cases passed. There were zero tool skips, failures or cancellations. Each phase ran through the pinned workspace-template Windows Job supervisor with pre/post source guards and 240-second management timeout; separate postguards passed, and the closing process inventory found zero owned survivors. The product authorization lifetime stayed five seconds.

## Actual commands and results

Cargo 1.98.1; Node 24.11.0; Java 21.0.8. Qualified phase command cwd was the existing gate marker under `E:/github/NearFuture/.tmp/issue-completion-gates/main/`; absolute manifest and npm `--prefix` selected the delivery worktree. This avoids assuming npm exec changes command cwd.

```powershell
$vault = "E:/github/NearFuture/.tmp/supplies-vault-worktree"
$manifest = "$vault/Cargo.toml"
$target = "E:/github/NearFuture/.tmp/supplies-vault-target"
node G:/dev/nvm4w/nodejs/node_modules/npm/bin/npm-cli.js --prefix $vault ci --offline --ignore-scripts --no-audit --no-fund
cargo fmt --all --manifest-path $manifest -- --check
cargo clippy --workspace --all-targets --locked --offline --manifest-path $manifest --target-dir $target -- -D warnings
node G:/dev/nvm4w/nodejs/node_modules/npm/bin/npm-cli.js --prefix $vault run check:types
node G:/dev/nvm4w/nodejs/node_modules/npm/bin/npm-cli.js --prefix $vault run check:boundaries
node G:/dev/nvm4w/nodejs/node_modules/npm/bin/npm-cli.js --prefix $vault run check:schemas
node E:/github/NearFuture/.tmp/issue-completion-gates/main/run-tools.cjs
```

The tool launcher executes `node --test` against all 24 files expanded from the unchanged `test:tools` script with the delivery worktree as cwd, including the real JFR file. It uses `execFileSync` within the qualified Job. Java 21 `JAVA_HOME` and its bin directory were explicit for Java-dependent tooling; no detached fallback or synthetic JFR substitute was used.

Focused Cargo commands (absolute paths retained to identify the exact tested cut):

```text
cargo test -p nf-store --test supplies_authorization --test supplies_burn --test supplies_burn_rejection --test supplies_compact_diagnostic --test supplies_crashes --test supplies_dedupe --test supplies_issuance --test supplies_provenance --test supplies_rejection --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --test supplies_rejection_controls --test supplies_rejection_replay --test supplies_replay_refusal --test supplies_reserve --test supplies_reserve_controls --test supplies_reserve_drained --test supplies_status --test supplies_status_branches --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --test supplies_conservation --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-kernel --test supplies_reserve_codec --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
```

Reports are retained under `.tmp/issue-completion-gates/main/`; hashes identify the complete captured command/output envelopes. Distinct postguards and 42-case aggregation are in `terminal-result.json` (SHA256 `af1eb974ccb032d996a91c6e00c62d9f086af272fc950f6a0fe20d6f0a8ae7c5`).

| Phase | Result | Captured report SHA256 |
|---|---|---|
| setup | PASS | `1d3d8f0456e4a4b9b3ef51323971fdcb6d5a1ce792be31e016622f2c3a68f35f` |
| quality | PASS | `1d0a7935642063e30dd7a079ea20acfe904ebab73f9ffbe0f3329c56bdc8fcdc` |
| supplies1 | PASS | `5f678fe7c25dc31a693ed76fec5db30c9841abe1b6c435e4198e83af094dee12` |
| supplies2 | PASS | `13f5a1218a3fba511698976aa10840e9de135d1626c60ccfced2794da97f5875` |
| conservation | PASS | `793927e2eb4d35e0fd7401153bcca26b41084d92582c8d9f981ef3d5679faeed` |
| kernelSupplies-corrected | PASS | `07ccc94e2df29e17ddd8e0bab859a70bfc3fbc221c9e0e47d6416c84082c0521` |

Source cases: [independent conservation](../../crates/nf-store/tests/supplies_conservation.rs) and [reference model](../../crates/nf-store/tests/supplies_property_support/reference.rs); [owned-worker crash/retry](../../crates/nf-store/tests/supplies_crashes.rs); [replay refusal](../../crates/nf-store/tests/supplies_replay_refusal.rs) and [terminal-decision corruption controls](../../crates/nf-store/tests/supplies_rejection_replay.rs); [authorization](../../crates/nf-store/tests/supplies_authorization.rs); [provenance](../../crates/nf-store/tests/supplies_provenance.rs); [reservation refusal](../../crates/nf-store/tests/supplies_rejection.rs), [burn refusal](../../crates/nf-store/tests/supplies_burn_rejection.rs) and [signed admission/conflict controls](../../crates/nf-store/tests/supplies_rejection_controls.rs); [kernel codec boundaries](../../crates/nf-kernel/tests/supplies_reserve_codec.rs).

## Broad verification and remaining hosted checks

Complete Windows verification passed: 398 distinct default Rust-harness cases plus 2 separately invoked actual Java integration cases, for 400 unique cases total. This includes the 42 supplies cases already reported. The broad core group repeated kernel 4; those repeated executions add no unique cases. Conservation was not repeated. The 2 default-ignored Java cases were executed in their dedicated opt-in phases. Management-only worker dispatchers are not behavior cases. Standard synthetic Java Gradle checks and the full workspace release build also passed. No fresh Java compilation claim is inferred from Gradle cache reuse.

All 12 accepted broad/setup phases and their 12 distinct postguards have complete qualified captures with status 0 throughout and no timeouts; final source guards preserved 849 tracked/source files, 800 unowned Main files and 957 user-workspace files. Final owned-process inventory found zero survivors. Full terminal aggregation: `.tmp/issue-completion-gates/main/full-terminal-result.json`, SHA256 `94e79c243004e622e929873e703b3909ca3fb987de08dad01343aee633c547a5`.

| Phase | Result | Rust-harness cases / artifact scope | Captured report SHA256 |
|---|---|---|---|
| core-captured | PASS | 155 | `65063313d1dbc0da7bd05568c6bf29c052c9f0493d8814f7ce12f8da99dc4813` |
| identity-captured | PASS | 36 | `f8a481fe78cd013f1dca04e40e3a5c8cccfe22490ece515cc6aebf38abb6c6bd` |
| ipcShadow-captured | PASS | 48 | `ef6318754cf1fb25d25f26408a7361fa50c5a374388a1303cc4774338c789e25` |
| storeLibDoc-captured | PASS | 2 | `0fdb07cba2e22844d824e735a5311fd58558f7be461f1c68e69a7689c1386e5a` |
| storeLegacy1-captured | PASS | 15 | `95375afce04ec2ec8e28c7fcb814c8c963afe834589dad8b18c1c12ea79495cc` |
| storeLegacy2-captured | PASS | 25 | `c49837dea2b85d1679e699f77b96e992f3a119b0b1a7338ea43cc71932ab7b5d` |
| storeLegacy3-captured | PASS | 11 | `700486f0707afdca161c058ae6df5b3d425d5e5fd63156b18bba7a23e7cdc5c1` |
| transport-captured | PASS | 68 | `60220039444a5602b2abe9788b152b090b48f99601089dce53cc6e047bf5b046` |
| javaSynthetic-artifact-setup | PASS | Gradle checks | `6ac50fce2eea53ef4e7500249818f0e1dcb9834501939d55203058c390e36da8` |
| javaInterop-artifact-setup | PASS | 1 | `e647d425c37f39c5f036caf2a6fa9046c41b1a396574ff3ef2e24650d3787492` |
| javaShadow-artifact-setup | PASS | 1 | `bb0db7660d1714baabb8540cf28b64edb7b11368cac8ddb00fc8f7fd97b7d8fe` |
| release-artifact-setup | PASS | Release build | `1cbaf3678d894976058536e29717113f93c61e828cbae3e97dcc93272d8b8b0a` |

Exact additional commands (same absolute delivery manifest/target and qualified phase cwd as above):

```text
cargo test -p near-future -p nf-contract -p nf-wire -p nf-kernel -p nf-nex-boundary -p nf-world -p nf-world-driver --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-identity --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-ipc -p nf-nex-shadow --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --lib --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --doc --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --test bound_query --test crashes --test failures --test fsync_failure --test identity_policy --test initialization --test membership --test miniature_advance --test miniature_all_actors --test miniature_authority --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --test miniature_bootstrap --test miniature_cancel --test miniature_crashes --test miniature_fsync_failure --test miniature_genesis --test miniature_independent_extended --test miniature_independent_vectors --test miniature_prepare --test miniature_profile_isolation --test miniature_resources --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-store --test miniature_resume --test miniature_timers --test miniature_writer_exclusion --test recovery --test transactions --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo test -p nf-transport --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target -- --test-threads=1
G:/dev/Android/AndroidStudio/jbr/bin/java.exe -version
node G:/dev/nvm4w/nodejs/node_modules/npm/bin/npm-cli.js --prefix E:/github/NearFuture/.tmp/supplies-vault-worktree run check:java
G:/dev/Android/AndroidStudio/jbr/bin/java.exe -classpath E:/github/NearFuture/.tmp/supplies-vault-worktree/java/gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain -p E:/github/NearFuture/.tmp/supplies-vault-worktree/java :ipc:exportAuthTestClasspath --no-daemon
cargo test -p nf-ipc --test java_process --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target -- --ignored
G:/dev/Android/AndroidStudio/jbr/bin/java.exe -classpath E:/github/NearFuture/.tmp/supplies-vault-worktree/java/gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain -p E:/github/NearFuture/.tmp/supplies-vault-worktree/java :nex-reference:exportDifferentialClasspath --no-daemon
cargo test -p nf-nex-shadow --test differential --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target -- --ignored
cargo build --workspace --locked --offline --release --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
```

Java tasks used `JAVA_HOME=G:/dev/Android/AndroidStudio/jbr` and its bin directory first in PATH. Actual interoperability used `NF_IPC_JAVA=G:/dev/Android/AndroidStudio/jbr/bin/java.exe` and `NF_IPC_JAVA_CLASSPATH_FILE=E:/github/NearFuture/.tmp/supplies-vault-worktree/java/ipc/build/auth-test-classpath.txt`. Differential used the same exact Java executable in `NF_SHADOW_JAVA` and the Main `java/nex-reference/build/differential-classpath.txt` in `NF_SHADOW_CLASSPATH_FILE`. Gradle retained `--no-daemon`.

The original core, identity, IPC/shadow and store lib/doc captures were overwritten by their postguards because of a launcher filename-expression error; live PASS summaries lacked complete outputs and were never accepted. The owned chain stopped during legacy store 1. Corrected named capture arguments, independent string report names and no-overwrite checks recovered those missing inspections once. Only the new complete `-captured` reports above supply acceptance. The earlier partial/collision evidence remains in `broad-reporting-failure-result.json`; the 42 supplies and 85 tool proofs were unaffected.

The first opt-in Java interoperability case failed with status 101 (one test failure, no timeout), and its separate postguard passed. The fresh exported classpath pointed to absent `java/wire/build/libs/wire-0.1.0.jar` and `java/contract/build/libs/contract-0.1.0.jar`. Matching Java sources from an earlier worktree did not establish that these runtime artifacts existed in Main. The normal `check:java` synthetic task generated the missing JARs, after which the unchanged interoperability case passed. The failed `javaInterop-captured-result.json` (SHA256 `faf4838a4840698883504845c30b98bdca1d64766cbb9a22ed5fb0866a44674f`) remains preserved; the accepted artifact-setup capture is separate. This was necessary artifact setup, not a product patch or an unchanged flaky retry.

## Hosted follow-up and CI repair

At published revision `05274d7cd4fc5c892016e5b74751173a4108b2c2`, both [push Linux](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37697857308/job/113053900295) and [PR Linux](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37697880658/job/113053981023) passed, including mandatory SQLite sync-fault recovery and release. [Push Windows](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37697857308/job/113053900543) failed the unchanged identity unit test's empty-input PowerShell `exit 0` probe before its large-pipe test or supplies tests. The reported `PrivateStorage` error did not identify its cause. The independent [PR Windows](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37697880658/job/113053980580) passed full checks, actual Java IPC and differential tests, then was cancelled during release. GitHub's annotation explicitly reports the maximum execution time of 20 minutes; release completion is not accepted for that run.

Follow-up code revision [`ff036884021791a45028097e39e6d1a129bc1ab7`](https://github.com/mirkoEscobedo/NearFuture/commit/ff036884021791a45028097e39e6d1a129bc1ab7) changes exactly three existing Main paths. The helper now retains only a closed private static failure reason, and its original positive test displays that reason if it fails. Public errors, arguments, five-second lifetime, pipe bounds and ownership/cleanup remain unchanged. No path, output bytes or OS error strings enter diagnostics. The original hosted probe's cause remains unobserved; no timeout or generic flake diagnosis is inferred.

The same release command now runs in an independent Windows/Linux matrix job with the same pinned checkout, Rust 1.98.1 and locked dependency fetch. Both verification and release jobs retain 20-minute limits. Full checks, actual Java tests and both Linux fault phases remain present. The split addresses the observed combined-job ceiling; hosted completion of the new jobs remains required.

Focused local verification of the changed helper passed all 5 existing identity unit cases, with zero ignored/filtered cases, followed by full workspace/all-target Clippy with warnings denied. Owned formatting and distinct postguards passed under qualified Windows Jobs, with no timeouts and zero owned survivors. These five cases were already part of the earlier 400-case cohort and are not added to its unique count. The earlier complete 400-case proof belongs to the original cut; a new complete local runtime pass is not claimed for the helper follow-up. All 47 supplies implementation files and 957 user-workspace files remain unchanged. The three explicitly reviewed follow-up paths leave 797 of the original 800 unowned Main paths untouched.

```text
cargo test -p nf-identity --lib --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target
cargo clippy --workspace --all-targets --locked --offline --manifest-path E:/github/NearFuture/.tmp/supplies-vault-worktree/Cargo.toml --target-dir E:/github/NearFuture/.tmp/supplies-vault-target -- -D warnings
```

Retained aggregate: `.tmp/issue-completion-gates/main/identityDiagnostic-terminal-result.json`, SHA256 `6f22aeecd668bdf90303ee637903e5f463ddb94487286dd459a78807b0d50684`; actual test/Clippy capture SHA256 `18143a0a06368b29862126aaa5a45802a3f7d4c076694065414fcb259773b6f6`. The initial formatter graph passed a directory and file suffix as separate arguments and failed before edits. Its source guard and closing postguard passed; corrected explicit absolute arguments passed once. Both captures remain retained as setup evidence, not behavior failures. The historical published manifest remains separate from the current source pins.

New exact-head Linux and Windows workflow acceptance remains pending. Earlier private-ACL failures on the separate ee1/PR65 cut are not failures of this Main cut: Main's unchanged default identity and transport suites passed locally. No in-transaction crash, power-loss, Starsector/native cargo, game save or deployment acceptance is inferred. Unsupported schema markers 2/3 test fail-closed current-shape refusal only, not historical migration.

Reviewed hosted-result links and any final delivery/documentation revision should be added before issue closure.
