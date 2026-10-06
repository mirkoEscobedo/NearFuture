# NF-002 foundation evidence

Checkpoint: 2026-10-07. This records the bootstrap checks before the dependent NF-003 codecs are integrated. GitHub acceptance must link the reviewed implementation revision; files alone are not a closure signal.

| Lane / command | Observed result |
| --- | --- |
| `npm ci --offline --ignore-scripts --no-audit --no-fund --cache .tmp/clean-npm-cache` | PASS; four vendored packages installed with an initially empty cache. |
| `cargo test --test node_cli --locked --offline` | PASS; two CLI behavior tests. |
| `npm run check:types` | PASS; strict no-emit JavaScript check. |
| `npm run check:boundaries` | PASS; Cargo dependencies and production Rust/Java source boundaries. |
| `npm run check:schemas` | PASS; fixture reproducibility only, eight positive/21 malformed/seven binding/six required-semantic fixtures. |
| `npm run test:tools` | PASS; 16 tests: four architecture, two unavailable-game lane, ten runtime/provenance checks. |
| `npm run check:java` | PASS; dependency-free contract compile, assertion test, reproducible JAR. |
| `java/gradlew.bat -p java gameAdapter -PgameDir=<selected licensed installation> --offline --no-daemon` | PASS; API compilation against the observed Starsector 0.98a-RC8 installation. |
| `java/gradlew.bat -p java syntheticCheck -PtestJavaExecutable=<selected installation>/jre/bin/java.exe --offline --no-daemon` | PASS; synthetic contract executes on the installed Java 17.0.10 runtime. |
| `javap -verbose <adapter class output>` | PASS; class-file major version 61 (Java 17), public BaseModPlugin superclass, no reflection reference. |

Tools actually used: Rust/cargo 1.98.1, host JDK 21.0.8, Gradle 8.14.3, Node 24.11.0, npm 11.7.0, TypeScript 5.9.3. Gradle wrapper SHA-256 and dependency locks are committed; npm tarball integrity is recorded in `package-lock.json`; Cargo dependencies are locked. Compiler output targets Java 17. See [runtime provenance](runtime/README.md) for game/mod versions and unsupported/unknown capability status.

The game lane packages only Near Future classes and reads the licensed API JAR in place. Neither build success nor running the synthetic Java test proves game classloader, campaign threading, saving, native-transfer safety, or capability G1–G5. No game was launched or modified. Public GitHub Actions execution must still be checked on the implementation PR; Windows local checks cannot establish an observed Linux run.

Observable red/green cycles: the initial node `--version` test failed with `Hello, world!`, then passed with an explicitly synthetic identity; the unsupported-argument test failed with exit 0, then passed with exit 2; Java identity assertions failed with the placeholder identity and passed after implementation; database dependency, Rust standard-library network import and Java game/ambient-effect fixtures each failed before the architecture rule was implemented, then passed. The two game-lane failure tests execute Gradle and require its specific unavailable-library diagnostic rather than accepting arbitrary tool failures.

Production files and test files stay below the package's topology warning limits. No generic skill copies, future feature scaffolding, proprietary libraries, runtime captures or machine-specific paths enter this foundation. The aggregate workspace check is rerun after dependent codec work rather than treating an interim NF-003 formatting warning as bootstrap evidence.

Dependent NF-003 verification and the reviewed pure crypto dependency exception are recorded separately in [Java contract evidence](protocol/java-evidence.md). The table above preserves the earlier bootstrap checkpoint.
