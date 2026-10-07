# Verification evidence and residual gates

Observed on 2026-10-07: public war regression RED on missing evaluator API, then GREEN for threshold neighbors, abort/ended lifecycle, exact-class suppression and modifier bits. Selected-peace regression independently RED on missing API then GREEN for f64-adjacent chance/treaty boundaries, conditional draw consumption and immutable effect order. Bounded diagnostic public seam likewise RED before implementation then GREEN.

```powershell
cargo test -p nf-nex-shadow --locked --offline
cargo clippy -p nf-nex-shadow --all-targets --locked --offline -- -D warnings
cargo fmt -p nf-nex-shadow -- --check
npm run check:types
npm run check:boundaries
java/gradlew.bat -p java syntheticCheck --offline --no-daemon -PtestJavaExecutable=H:/Games/Starsector/jre/bin/java.exe
node crates/nf-nex-shadow/fixtures/generate-identity.cjs --check
```

The crate contains exact 10/16 shared TSV corpus tests, 26-case worker-count1/2/4 repeat/reordered completion checks, conservative copied-input bounds/nonfinite failure, input-identity whole bytes and SHA256 goldens/context changes, stale/lifecycle/provider-error fence tests, bounded/coalesced diagnostic behavior and explicit synthetic reproduction file round-trip. The default Cargo suite explicitly ignores the separately configured real-JVM integration; no skipped integration success is inferred from that suite.

The new Java test-only `DifferentialOracleMain` and readers use raw input columns and ordinary fixture defaults, then call the frozen source-adapted Java production methods. They never read the golden expected result columns and never import Rust algorithms. Output retains ordered raw modifier/effect bits and is capped. Existing reference production/tests are unchanged; only additive oracle/classpath Gradle tasks are introduced.

Actual installed Java17 versus Rust comparison passed for all26 cases through the package-native Windows kill-on-close Job Object owner. The unique local harness serially compiles/exports Java then runs the explicit ignored integration test; both structured commands reported `processOwnership: windows-job-object`, status0, `timedOut:false`, native verifier verdictPASS. Rust additionally owns each exact Java child with ten-second timeout, capped stdout/stderr, known-handle kill/wait, and checked owned temporary-root cleanup.

```powershell
$env:NF_SHADOW_JAVA='H:/Games/Starsector/jre/bin/java.exe'
$env:NF_SHADOW_CLASSPATH_FILE='E:/github/NearFuture/java/nex-reference/build/differential-classpath.txt'
npm.cmd exec -- workspace-template verify .tmp/shadow-review --scope module --module node:. --timeout 180000
# Structured steps inside that package-native harness:
# java -cp E:/github/NearFuture/java/gradle/wrapper/gradle-wrapper.jar org.gradle.wrapper.GradleWrapperMain -p E:/github/NearFuture/java :nex-reference:exportDifferentialClasspath --offline --no-daemon
# cargo test --manifest-path E:/github/NearFuture/Cargo.toml -p nf-nex-shadow --test differential --locked --offline -- --ignored --nocapture
```

The two baseline public corpus identities are SHA256 of the normalized Git blob bytes (LF): `war-v1.tsv` = `5a009c776b643172fea6889a4ffbd8d481856f5bc9b7126c8f2be8b43d66fe16`; `peace-v1.tsv` = `74954740ef83d1cc7b701a274519887644b50d646057f1c1e88538890db034d5`. Source/ref/config/runtime/implementation and full input identities are separately required in bound shadow evaluations; synthetic test metadata is deliberately public and does not establish native runtime verification. Reviewed implementation revision: ``6907179bd4f8b09366050f57bfa6522215271482``.

This portable slice does not close issue15. Missing actual game capture/writer coverage, proprietary MutableStat total priority/order, multi-target/player decision branches, complete action selection/lifecycle, live side-effect success, source/JAR equivalence and native disconnect/save/load hooks remain explicit promotion blockers. No private campaign or proprietary data is included. Capture and projection remain SHADOW_ONLY.

## Independent R1 binding repair (one semantic round)

The evaluator-only reviewer separately passed the selected war/peace arithmetic, bounds and real26-case Java17 oracle, while the coordinator found that guarded acceptance could not receive a changed current-owner binding. Public regression RED observed old acceptance returning `Ok(War(...))` after the owner's frontier advanced with identical numeric facts. Another public regression RED observed world-derived result acceptance despite no whole-Snapshot commitment.

The single R1 repair adds explicitly validated current metadata to acceptance and requires exact original/result/current binding plus current input commitment. GREEN regressions cover changed SID, frontier, merged config, source, capture policy, ruleset, subject and instance with unchanged facts, and invalid current metadata. The world factory now privately marks its inspectable result `WorldWithoutCommitment`; guarded acceptance returns `MissingFact`. The copied-facts primitive remains diagnostic-only. No full-world encoding, read-set certification, wire registry or native authority was added. Fresh full crate checks passed (14 public tests), formatter/strict Clippy and independent fixture regeneration passed, and the separately invoked Java17 differential test passed all26 raw-input cases under the package-native Windows Job Object owner. Fresh repository type and boundary checks also passed. The coordinator independently inspected the repaired owner-binding and world-scope guard, complete private profile, diagnostics and Java oracle. R2 returned scoped PASS; a fresh native-supervised crate lane passed all14 public tests plus the explicitly invoked real Java17 26-case differential test. No second semantic repair was needed.

## Isolated publication gate

The accepted-baseline-plus-shadow candidate passed142 standard Rust tests and40 Node checks, formatter, warning-denying all-target Clippy, types, boundaries, all fixture regeneration and Java synthetic checks. Additive Java classpath exports passed. Explicit actual bundled Java17 interoperability passed: control/bulk sockets13.90s and all26 raw-input Java/Rust differential cases0.29s. The locked offline workspace release build passed. All five successful structured steps reported processOwnership=windows-job-object, status0 and timedOut=false. [Hosted Windows/Linux gates](https://github.com/mirkoEscobedo/NearFuture/actions/runs/37553297007) passed, including mandatory actual Java21 socket/differential tests and locked release builds. [PR #56](https://github.com/mirkoEscobedo/NearFuture/pull/56) merged as ``03d500dc7e975b440e446f2f840a73d5f1e809d7``.

A clean-checkout fixture check first exposed automatic CRLF conversion of the TSV; the narrow LF attributes policy resolved it without changing the private profile bytes. The full check and export then passed; a separately invoked interoperability command stopped at setup because the coordinator used the wrong classpath variable name. Correcting NF_IPC_JAVA_CLASSPATH_FILE and explicitly running both process tests and release produced native verifier PASS. No failed setup is counted as interoperability evidence, and no default ignored test is treated as passed.
