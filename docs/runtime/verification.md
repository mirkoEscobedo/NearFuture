# NF-001 verification evidence

Work date: 2026-10-07 (Europe/Rome). Issue: [NF-001 / #5](https://github.com/mirkoEscobedo/NearFuture/issues/5). Working revision is pending the coordinator's integration commit; this evidence does not claim issue closure before independent review.

| Command/check | Result and limit |
|---|---|
| `node --test tests/runtime-manifest/capture.test.mjs` | Initial RED: public capture module absent; GREEN: same fixture yields identical output, exact independent SHA-256 `abc` vector, correct versions and private-path/secret exclusion. |
| `node --test tests/runtime-manifest/diagnostics.test.mjs` | Initial RED: unverified-JVM/unknown-build recovery diagnostics absent; GREEN after diagnostic policy. A further RED exposed null enabled-mod input as a TypeError; GREEN added document-shape validation. Covers missing API, config hash changes, enabled-mod recovery and traversal rejection. |
| `node --test tests/runtime-manifest/metadata.test.mjs` | RED: installed mods' single-quoted version metadata rejected; GREEN after non-evaluating parser support. Separate RED: arbitrary `-XX:PrivateKey=secret` escaped redaction; GREEN after limiting string options. |
| `node --test tests/runtime-manifest/*.test.mjs` | 10 passed, 0 failed/skipped; fixture and CLI tests use Node built-ins. Production code under 200 lines per module; each behavior test file under 100 lines. |
| `npm run check:types` | Strict checkJs/noEmit passed over new ESM/JSDoc and repository tools. |
| Twice: `node tools/runtime-manifest/cli.mjs --game-dir $env:STARSECTOR_HOME --game-version 0.98a-RC8 --output <local-json>` | Real installed game captured twice with identical SHA-256 `ff87b70b0648d44d23e47f1edff912e228e62bd3eed8e9f871944c33f9b08e34`. 114 enabled mod records, 1,843 artifacts, no missing/ambiguous active mod diagnostic. Output contains hashes/metadata only. |
| Public `javap` signatures and `Version` constants | Game build and candidate hook API shape inspected; no game execution, implementation decompilation or compatibility certificate. |
| GitHub immutable tag/source inspection | Nex tag commit and licensing exceptions inspected; no third-party source/assets copied. Source-to-installed-binary equivalence unverified. |

The observed manifest is reproducible on unchanged installation/host/configured launcher input. It deliberately carries no timestamp or absolute paths. It is not a complete runtime content digest, an atomic filesystem snapshot or an economic admission certificate. Hashes of configuration also cover private-valued settings without publishing their contents; selected safe JVM flags are displayed and original launch files hashed.

Game integration E-SAVE/E-THREAD/E-CONSOLE/E-NEX-WRITER/E-NEX-ORACLE/E-BRIDGE/E-SHELL/E-OWNERSHIP and G1–G5 remain blocked pending an adapter and disposable campaign test harness. No supported/experimental entry is manufactured from mocks or JDK compilation. Repository quality checks and prohibited-content scan are run at coordinator integration; no game JAR, asset, private save or credential was added by this issue slice.
