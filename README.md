# Near Future

Near Future is an external strategic-runtime project for Starsector. This foundation contains a Rust synthetic node, a pure data contract crate, a pure Java contract with strict maintained Ed25519 validation, and a minimal Java API adapter. The node reports its build identity; it does not host a world or certify game integration. The roadmap and design are tracked in GitHub issues #1–#4.

## Pinned tools

| Tool | Version / policy |
| --- | --- |
| Rust | 1.98.1, `rustfmt` and `clippy`; `rust-toolchain.toml` |
| Build JDK | 21.0.8; `.java-version`; compiler output targets Java 17 |
| Gradle | 8.14.3; wrapper distribution verified by SHA-256 |
| Node | 24.11.0; `.nvmrc` |
| TypeScript checker | 5.9.3; vendored archive and npm integrity lock |

The locally observed game runtime is Java 17.0.10. Running the synthetic contract on it verifies that contract's bytecode compatibility only. The supported runtime and unresolved capability gates are recorded in [runtime documentation](docs/runtime/README.md).

## Windows contributors

Install the pinned Rust toolchain, Node and JDK. Set `JAVA_HOME` to JDK 21.0.8 and ensure `java`/`javac` resolve to it. From the repository root:

```powershell
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
npm ci --offline --ignore-scripts --no-audit --no-fund
cargo fetch --locked
npm run check
cargo run --locked --offline -- --version
```

All npm dependencies are checked-in `vendor/` archives. The existing Windows `workspace-template` tooling remains available through `npm exec -- workspace-template`; its release/executable provenance is recorded in `.agentic/project.json`. It is an optional Windows package so Linux contributors can install the portable checks. A first Rust fetch and Gradle-wrapper invocation need Internet access for locked open-source build dependencies; subsequent cached builds can use `--offline`. No public command downloads Starsector or mod binaries.

`npm run check` runs Rust formatting/linting, strict no-emit JavaScript type checks, dependency boundaries, schema-fixture reproducibility, Rust and tooling tests, and the synthetic Java contract. Java tests are small executable assertion programs and do not require a test-framework download. `cargo build --workspace --locked --offline --release` builds the headless node. Each component is also exposed as a named `package.json` command.

## Headless Linux contributors

Use the same tool versions and root commands as Windows. The Java script calls the committed Gradle wrapper through `sh`, so an executable file-bit change is unnecessary. The platform-specific workspace-template package is skipped on Linux. Public CI runs the complete synthetic check and release node build on both Windows and Linux; a green synthetic check does not imply a green game capability gate.

## Opt-in licensed game lane

Set `STARSECTOR_HOME` to your own licensed installation, then compile the adapter:

```powershell
$env:STARSECTOR_HOME = '<your licensed Starsector installation>'
npm run check:game
# Or pass the installation explicitly:
.\java\gradlew.bat -p java gameAdapter '-PgameDir=<your licensed Starsector installation>' --no-daemon
```

The required local input is `starsector-core/starfarer.api.jar`. Missing configuration or libraries produces a nonzero exit and an actionable `Game integration unavailable` diagnostic. This lane references the input in place and packages only Near Future classes; it never copies, downloads, or republishes the game library. It proves API compilation, with no in-game execution claim. Public CI never enables this lane against a licensed installation; it does check the missing-input failure behavior.

To run the synthetic Java contract using the bundled game JVM after compiling with the pinned JDK:

```powershell
.\java\gradlew.bat -p java syntheticCheck "-PtestJavaExecutable=$env:STARSECTOR_HOME/jre/bin/java.exe" --no-daemon
```

Capture a private local provenance manifest separately:

```powershell
node tools/runtime-manifest/cli.mjs --game-dir $env:STARSECTOR_HOME --game-version 0.98a-RC8 --output .tmp/runtime-manifest.json
```

Manifest capture is read-only and preserves unknown/unavailable gate status. `.tmp/` is ignored. Never commit game libraries, saves, runtime captures, private machine paths, or secrets. The adapter deliberately uses public APIs; game classloader and campaign-thread behavior still require real game validation.

## Dependency direction

The node owns process/environment/output effects. `nf-contract` is `no_std` with reviewed, exact-version pure hashing, normalization, and Ed25519 dependencies; its public values and decisions cannot import `std` network/storage APIs. Java contract dependencies permit only pinned Bouncy Castle 1.81 pure Ed25519 point validation; standard JDK primitives hash and sign. Contracts cannot import game, network, database, clock, or reflection APIs. The Java adapter is the only game-API seam. `npm run check:boundaries` inspects Cargo metadata and production sources; Gradle also rejects dependencies outside that exact pure crypto exception. These are architectural checks, not a hostile-code sandbox. New volatile effects belong in adapters/ports when an implemented use case requires them.

Use `npm exec -- workspace-template skills list` to discover package-owned methodology. Skills stay in the package rather than being copied into this repository. The package's alpha `update status` reports `INCOMPLETE` for local `file:` packages; `doctor` inspects their installed assets and integrity.
