# Neutral ACK32 fixture preparation recipe

Refs #99. This is a separate, closed **preparation** format. It does not change
legacy fixture/seed readers, launch/input guards, clocks, frontends or native
authority. The old guards intentionally reject its preparation receipt.

`NEUTRAL_ACK32_TOT_NEX_V1` fixes Starsector0.98a-RC8, Windows x86_64 Azul
17.0.10+7-LTS, companion0.1.0 and neutral-probe-v1-ack32. Its installed dependency
closure is exactly MagicLib1.5.6, ToT1.0.11, Ashlib2.2.3, BMO2.1.0, LazyLib3.0.0
and Nexerelin0.12.2c; the prepared enabled list adds near_future_probe0.1.0.
It neither accepts arbitrary mods nor ignores missing/version-mismatched mods.

The caller supplies a private manifest and its SHA256. The exact top-level
fields are schemaVersion(1), profile, gameVersion, protocol, companionVersion,
sourceInstallation, packageRoot, hostRoot, scenarioRoot, runtime, package, host,
mods and protected. All roots are absolute ordinary directories; scenarioRoot
is a direct child of this checkout's ignored `.tmp/starsector-proxy/scenarios`.
Inputs must not overlap that scenario. JSON keys are unique and closed; manifest
size/depth, relative paths, individual bytes and tree inventories are bounded.

Each file pin has exactly path, bytes and lowercase sha256. Paths are contained
relative paths with ordinary ancestry. Runtime pins identify the core API,
ToT-supplied0.98a patch, jre/release, jre/bin/java.exe and
jre/bin/server/jvm.dll. Core and bundled API bytes must match. Release properties
must match the named JVM. The recipe verifies caller-supplied binary identity;
it does not authenticate an author release, rebuild proprietary sources or prove
that a patched class verifies/loads in the game.

Package pins identify probe-package.json, mod_info.json and exactly the two
companion JARs. Generated package metadata must retain experimental/disabled
status, Java17 target, pinned compiler, ACK32, one outstanding request and eight
requests per binding. Declared entrypoint/JARs and artifact digests must agree.
The host record pins nf-sidecar.exe, its source commit and ACK32. Use the final
reviewed #139 host artifact, not the preserved ACK16 or early probe bundle. A
compiled snapshot module grants no game authority; the pinned companion requests
the probe capability only.

Each of the six mod records has exactly id, version, directory, metadata, jars
and dependencies. Metadata identity, version, declared JAR list and dependency
closure must match the named profile. JAR directory inspection refuses entries
under com/fs/starfarer/api; no second core API is staged through a mod JAR.
Multi-release entries under META-INF/versions/<n>/com/fs/starfarer/api are also
refused. This ZIP-directory check is not bytecode, signature or loader certification.

Protected pins cover exactly every current original save file plus vmparams,
mods/enabled_mods.json, core settings and compiler directives. Changed bytes,
extra/missing save files and links refuse. Original autosave must remain enabled,
default-on, retention3. Preparation uses exclusive-create backups, verifies every
copy and rechecks the original pins/census afterward. Existing or partial output
is preserved and refuses replacement. No original config/save is written.

```powershell
node tools/starsector-proxy/fixture/neutral-recipe-cli.mjs inspect <private-manifest> <sha256>
node tools/starsector-proxy/fixture/neutral-recipe-cli.mjs prepare <private-manifest> <sha256>
```

`inspect` is read-only. Explicit `prepare` creates only neutral-preparation:
verified protected backups, prepared vmparams/settings/enabled-mod files and a
preparation-only receipt. It copies no installation, runtime, mod assets or seed,
starts no process/provider and claims no native permission. It retains original
retention while disabling autosave in the private settings copy. Protected
settings keys are counted after decoding quoted/escaped spelling; duplicates,
unquoted aliases and nonmatching scalar values refuse. Private launch
construction removes verification-disabling flags, adds -Xverify:all, rewrites
all four game data paths inside the scenario, and supplies the explicit host pin
and fresh private evidence destination. Inherited agents/probe flags, indirect
@argfiles/XX:Flags/XX:VMOptionsFile and unsafe launcher paths refuse. Config
construction is not actual JVM/game verification; future launch admission must
also verify effective arguments and environment option sources.

## Seed identity without fabricated provenance

```powershell
. tools/starsector-proxy/fixture/neutral-seed.ps1
Read-NeutralSeedIdentity <candidate-directory> <save_directory> <private-manifest> <sha256>
```

The reader first validates the pinned recipe. It then reads exactly two bounded
plain XML files with DTD/external resolution disabled, requires the native roots,
save identity/fleet reference, uncompressed matching game build, and exactly
seven declared mod identities/versions. It resolves the observed EnabledModData
spec/versionInfo references through unique descriptor IDs; dangling, cyclic,
ambiguous, missing, duplicate or extra identities refuse. It never rewrites a
descriptor to make it match, copies a candidate or invokes an ignore-missing-mod
control. Return values explicitly say nativeCreatedProof=false and
launchAdmitted=false. A synthetically constructed descriptor can pass identity
inspection; that is not evidence of native creation or a usable seed.

Still required separately: a genuine disposable seed created on this exact
profile with qualified source/creation/save/process/image custody, runtime asset
staging and launch admission, actual verified loader permission, current finite
caller authority and ordinary-exit tail. Pause/combat/save-reload action support
is held. No actual installation/seed copying, game launch/input or native proof
occurred while implementing/testing this recipe. Tests use tiny generated
artifacts/XML and retain unchanged legacy behavior.
