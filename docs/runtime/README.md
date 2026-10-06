# Runtime inventory and support status

NF-001 records compatibility inputs before any strategic authority is enabled. The observed installation is **unknown / legacy-read-only**. No live campaign hook, save durability, Console event delivery, Nex substitution or G1–G5 gate has passed. The matrix deliberately contains no supported or experimentally supported configuration; inventing a certification would conceal the remaining experiments.

Run with Node 24 (built-in modules; no game launch or save access):

```powershell
node tools/runtime-manifest/cli.mjs --game-dir $env:STARSECTOR_HOME --game-version 0.98a-RC8 --output .tmp/runtime-manifest.json
node --test tests/runtime-manifest/*.test.mjs
```

Use an owner-supplied `STARSECTOR_HOME` path. Near Future reads `starsector-core/starfarer.api.jar` and the locally installed dependencies in place; proprietary artifacts must not be copied into this repository. `--help` describes the input contract. A missing installation/API/JVM/launcher input fails with a recovery diagnostic. Unknown JVMs/builds and missing/ambiguous active mod IDs stay legacy/read-only. The game build is explicitly declared by the contributor; compare it with the game's version display or public `com.fs.starfarer.Version.versionOnly` constant. A mod's `gameVersion` is a requirement, not proof of the installed game version.

The [observed manifest](observed-installation.json) records actual SHA-256 hashes, game build, JVM release vendor/version, OS/architecture, configured JVM flags, enabled mod metadata and JARs, and root/data/config JSON/CSV configuration. Mod folder names become logical mod IDs/version paths. No timestamp or absolute installation path participates in the JSON, so repeated capture of unchanged inputs on the same host produces identical bytes. Comments, trailing commas and single-quoted metadata strings are parsed without evaluating code; arbitrary extensions fail with an actionable diagnostic.

Only hashes and metadata leave the installation. Logs, private saves, keys, launcher registration data, assets and binary bytes are excluded. Public flags include heap/stack sizes, numeric VM options and recognized runtime options. Arbitrary properties, agents, paths and unknown string values are omitted; the original launcher files are hashed. Treat manifest sharing as an explicit support export: mod IDs disclose which mods were enabled. Read output before publishing it.

Configured `vmparams` flags are not proof of the running process flags. Windows executable, batch launch and an external mod launcher may use different configurations; batch/command files are additionally hashed. This capture does not execute launchers, inspect a live process, validate JVM flag acceptance, or infer compatibility from a successful modern-JDK compile. The installed game runtime and script classloader need their own experiments; see the [Starsector debugging guide](../starsector-debugging.md).

The machine-readable [support matrix](../../config/runtime-support.json) distinguishes supported, experimentally supported and unknown configurations. Certification requires exact artifact identities and named capability evidence reviewed independently. Version strings alone never promote a configuration, and the capture tool always returns unknown; it cannot issue certificates.

The captured configuration is an inventory envelope, not a complete content/ruleset digest: graphics, sounds, hull/spec data outside `data/config`, native libraries, JVM library contents and arbitrary script source are not hashed. Any future authority or economic admission policy must extend the digest to its actual read domains. Files are read sequentially; freeze launcher/mod changes during capture. Cross-file transactional snapshots and hostile concurrent edits are not covered.

See [source provenance](source-provenance.md), [hook experiments](feasibility.md), and [verification evidence](verification.md).
