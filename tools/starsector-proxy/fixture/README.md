# Protected Starsector fixture tooling

The maintained fixture code lives here. Private bindings, plans, leases,
runtime manifests, source pins, licensed game copies, backups, saves, images,
logs, recordings and receipts belong under ignored
`.tmp/starsector-proxy/scenarios/<scenario-id>/`.

This is the current NearFuture native99 profile. It retains the existing
182-file / 178-save census, global autosave shape, six-jar/node runtime layout,
first-use-only preparation and 180s preparation, 60s ownership-probe and 480s
launch bounds. A new profile needs its own qualification; these scripts do not
grant input or turn a partial run into issue acceptance.

| File | Responsibility |
| --- | --- |
| `fixture-inputs.ps1` | Read the caller's hash-bound private inputs; reject path escape, reparse ancestry, changed inputs and unknown fields. |
| `prepare-fixture.ps1` | Copy and protect the source installation, disable autosave only in the private copy, stage pinned runtime inputs and initialize its private vault. |
| `seed-fixture.ps1` | Validate and stage one explicitly admitted generated disposable save; recheck it at both fresh launch gates. |
| `protection-check.ps1` | Verify runtime/fixture provenance, original and backup hashes, save census, private save state and explicit launch authority. |
| `run-native.ps1` | Invoke the maintained Windows Job supervisor for ownership-probe or launch. |
| `native-controller.ps1` | Own the launched process tree, record exact process identities, observe normal exit or request owned cleanup. |
| `publish-owned-snapshot.ps1` | Publish complete live metadata atomically, retaining the previous timestamp during a reader sharing conflict. |
| `read-owned-client-geometry.ps1` | Read current owned HWND client origin with `ClientToScreen`; require matching outer bounds, process identity, geometry resampling and current deadlines. |

The four fixture roles require `-BindingPath` and `-BindingSha256`.
The legacy version 1 private JSON remains valid and has exactly these fields:

- `schemaVersion`: `1`.
- `fixtureRoot`: an existing immediate child of the repository's ignored scenarios directory.
- `sourceInstallation`: the original local installation to protect.
- `qualifiedRuntimeRoot`: the qualified directory containing `jars/` and `diagnostic/`.
- `candidateHead`: the qualified 40-character Git commit.
- `runtimeManifestSha256`: the independently admitted digest of the fixture's `runtime-artifacts.private.json`.
- `presentationAgentPath`, `presentationAgentSha256`, `presentationClassSha256`: existing qualified profiling inputs.
- `driverRole`: the caller's finite driver identity, shared by the plan and lease.

Hash the completed binding before invoking a role. Do not edit a live binding,
plan or lease. Keep their historical bytes and every failed attempt.
The scripts resolve their maintained siblings from this directory; they do not
evaluate code from private run folders.

For the finite direct frontend, the frozen plan's `fixtureBinding` descriptor
contains exactly the absolute scenario-local `fixture-binding.private.json` path
and its SHA-256 (`path`, `sha256`). Its `fixtureRoot` must be that admitted scenario
and its `driverRole` must match the current lease. The provider/child independently
verify this binding before the ownership probe and recheck it afterward. See
`docs/starsector-proxy.md` for the complete clean-checkout dependency route.

## Immutable menu launch authority

New V1 menu batches require an explicit `launchGrant` descriptor in both the
frozen plan and `MENU_SMOKE_ONLY` lease. It has exactly `path` and `sha256`, naming
the scenario-local `menu-launch-grant.private.json`. This grant authorizes only
protected launcher entry; it does not change the meaning of either lease phase
or authorize campaign loading, saving, seeded entry or an owned-node kill.

Construct the files in this order to avoid a digest cycle:

1. Complete fixture preparation and freeze the existing fixture receipt/binding.
2. Freeze the support manifest returned from the complete provider input
   inventory, including `menu-launch-grant.mjs` and `fixture/menu-launch.ps1`.
   Keep installed runtime and executable pins. Exclude scenario authority files.
   Also create the scenario-local `source-pins.private.json` as a byte-identical,
   create-only copy of `frozen-support-pins.private.json`, and verify their
   digests. The controller unconditionally records this bookkeeping input,
   including for menu-only runs. The copy grants no owned-node-kill authority.
3. Write the launch grant with exactly `schemaVersion: 1`,
   `kind: "FINITE_MENU_LAUNCH_GRANT_V1"`, `phase: "CAMPAIGN_BINDING_ONLY"`,
   `authorized: true`, `issuer: "/root"`, `driver: "/root/recovery_game_driver"`,
   `scenarioId`, `fixtureBinding` (`path`, `sha256`), `fixtureReceiptSha256`,
   `supportPinsSha256`, `ownedGameHardTimeoutMs: 480000` and
   `campaignLoadSaveAllowed: false`. It contains no plan digest.
4. Freeze the plan with its fixture and launch descriptors, exact driver
   ownership, and existing 8-action/3-invocation/180000ms/480000ms bounds. Its
   batches are unique members of B1/B2/B3 and include the launcher batch B1.
5. Freeze the menu lease with the completed plan digest and identical launch
   descriptor. Either omit optional `sessionWatchdogMs` from both plan and lease,
   or declare the integer 480000 in both. The runner checks this paired presence
   before creating the supervisor; passing the launch protection or JS grant
   validator alone does not establish the runner's watchdog agreement.
   Recording-profile, seed, campaign and owned-node-kill extensions refuse.
6. Run the maintained protected launch. Both launch gates verify the same grant,
   actual binding pair and current tooling pins, plus all original protection
   and recording checks. The controller records the validated descriptor and
   actual frozen root-lease digest before starting the launcher.
7. Issue each original single-use B1/B2/B3 authorization only after obtaining
   its current owned-process receipt. The finite provider/child and mutation
   checks independently reread the grant and require the controller's original
   root-lease digest. A later replacement lease cannot inherit the launch.

No live plan, grant, binding or lease is rewritten during this sequence. The
source-composition regression exercises real PowerShell launch admission and
the real JS finite validator with generated protected files and process data;
it launches no game and does not qualify native input or ordinary exit.

This V1 composition does not qualify V2 paused-campaign handoff. V2 needs a
separately admitted immutable successor authority derived from actual postlaunch
witnesses; changing a V1 lease in place is not that handoff.

Run preparation through `workspace-template verify` with its existing 180s
owned Windows Job. After its actual PASS and protection checks, the runner's
`-Phase ownership-probe` retains the separate 60s probe. Launch requires a
current caller-issued plan/lease and one serialized driver. No script authorizes
an unplanned input. Original installation settings and saves stay untouched.

The geometry helper accepts `-EvidenceRoot -WindowId -ExpectedX -ExpectedY
-ExpectedWidth -ExpectedHeight`. Window capture dimensions alone do not identify
its pixel origin: the installed v7.1.0 upstream capture samples a client DC into
an outer-sized bitmap. Use the measured current client origin plus the original
image point, then verify the intended control in a fresh image. Never hardcode a
previously observed offset.

`fixture-inputs.test.mjs` exercises the real PowerShell input boundary without
launching Starsector. Current tests cover valid bindings, digest tampering,
private-root escape, changed agent bytes, unknown authority fields, runtime
manifest binding and live-process refusal with native Windows paths.

The relocated scripts passed seven offline tests, protected fixture preparation
and an ownership-only probe under the existing Windows Job supervisor. Real
launcher and game-menu captures also worked; campaign acceptance is incomplete.
A subsequent live run exposed metadata publication failing while a reader held
the current file open. The publisher now writes a complete same-directory
temporary file and atomically replaces the current snapshot. Only Windows sharing
or lock violations skip publication; other failures remain fatal. A skipped
sample retains the old timestamp, so the existing five-second freshness check
still refuses stale input. `publish-owned-snapshot.test.mjs` covers a real Windows
reader lock and fatal non-sharing failures; it launches no game.
Original per-run script snapshots remain historical evidence.

## Optional startup recording profile

A version 2 fixture binding uses the same fields, with `schemaVersion: 2` and
the additional required `recordingProfileSha256`. That digest pins the ordinary
`recording-profile.private.json` file directly under the same fixture root.
Version 1 retains its existing recording recipe and refuses this new authority
field in the binding, frozen receipt, launch plan or lease.

The profile is a closed object with exactly these values and digest fields:

- `schemaVersion: 1`, `kind: NATIVE99_JFR_STARTUP_PROFILE_V1`,
  `id: CLASSLOAD_EXCEPTIONS_V1`.
- `enabledEvents`: exactly `jdk.ClassLoad`, `jdk.JavaExceptionThrow` and
  `jdk.JavaErrorThrow`, each once; `stackTrace: true`, `classLoadThresholdMs: 0`.
- `recordingDurationSeconds: 420`, `recordingMaxSizeMiB: 64`.
- `baseDefaultJfcSHA256` and `gameJvmReleaseSHA256`: the source installation's
  `jre/lib/jfr/default.jfc` and `jre/release` digests. The staged copies must match.

The profile is limited to 16 KiB; unknown or duplicate fields and reparse ancestry
are refused. The version 2 frozen receipt keeps schema version 1 and records the
profile digest. The current launch plan and lease must declare the same digest.
Launch protection independently regenerates the complete expected JFC, preserving
the legacy recipe except for enabling those three events. Updating generated
receipt hashes cannot admit another recipe. The JVM declaration remains one exact
private recording: 420 seconds, 64 MiB, `capture/active.jfc` settings and
`capture/current.jfr` output under that fixture root.

A settings-only probe with the bundled Zulu 17.0.10+7 JVM accepted the generated
JFC and the 420-second / 64-MiB startup declaration during `java -version`. It
produced a recording readable by the bundled `jfr summary`. Live game recording
validation remains pending. Exception and Error events describe object creation;
they do not establish that an exception was uncaught or caused a delay. Fixture
tests and the settings probe do not establish native game behavior, performance
or campaign acceptance. A separately admitted live recording and its inspection
are still required.

## Optional generated disposable seed

The ten-field binding above is unchanged. Without explicit seed authority the
private save root must be empty. A seed claim or receipt also refuses an unseeded
launch, including a failed stage that copied no files. Staging does not authorize
a game launch or input, and `-GuardedResume` remains refused.

Prepare and qualify the empty fixture first. Its frozen receipt continues to
record the historical empty state and is never rewritten. A separate, hash-bound
`seed-manifest.private.json` under that fixture root has exactly these fields:

- `schemaVersion: 1`, `kind: NATIVE99_DISPOSABLE_SEED_MANIFEST_V1`.
- `sourceScenario`, `sourceEvidenceScenario`: immediate scenario directory basenames.
- `sourceRun`: the positive source run number.
- `saveDirectory`: the exact generated `save_` basename; `gameVersion`: the target version.
- `files`: exactly two `{relative, bytes, sha256}` rows for `campaign.xml` and `descriptor.xml`.
- `provenance`: exactly `fixtureReceipt`, `rootCall`, `closeout`, `protection`, `launch`, each `{bytes, sha256}`.

Source paths are derived from those names under the repository's ignored
scenarios directory. Evidence names are fixed: the source frozen receipt,
`root-call.actual.private.json`, `driver-closeout.actual<sourceRun>.private.json`,
`after-smoke-protection.private.json` and `launch-supervisor.private.json`.
The actual root wrapper and closeout session must agree. A reaped, protected
source run with a truthful supervisor `FAIL` is valid provenance; it is not
reported as a successful game check.

The source directory must contain only those two ordinary plain XML files, with
their exact lengths and digests and at most 16 MiB combined. All ancestry must be
free of reparse points. XML readers prohibit DTDs, set both resolvers to null and
cap document characters at 16 MiB. Validation requires `CampaignEngine` and
`SaveGameData` roots, matching `saveDirName`, one numeric `playerFleet/@ref`
resolving one `Flt/@z`, character data, and a present `saveDesc` (empty is valid).
The descriptor must declare uncompressed data, the exact target game version
and no enabled mods; it has no required `saveDirName` field. This validates the
serialization boundary, not the game's ability to load the campaign.

Staging uses a distinct immutable `seed-stage-plan.private.json`, whose
`seedStage` object is exactly `{mode: STAGE_ONE_PRIVATE_DISPOSABLE_SEED,
manifestSha256}`. The fixed `seed-stage-authority.private.json` has exactly
`schemaVersion: 1`, `kind: NATIVE99_SEED_STAGE_AUTHORITY_V1`, `authorized: true`,
`scenarioId`, `planSha256`, `fixtureReceiptSha256`, `sourcePinsSha256` and
`manifestSha256`. Here `planSha256` binds the stage plan, not the later launch
plan. Source pins must include the current input, preparation, protection and
seed modules. Invoke the existing preparation role with the normal binding pair
and the additional paired `-StageSeedAuthorityPath` / `-StageSeedAuthoritySha256`.
Use the existing 180s Windows Job supervisor.

Before the first effect, readonly `admit-seed-stage` checks frozen provenance,
empty saves, absent log, no live game processes and every original/backup/save
and autosave protection. Staging creates one claim with `CreateNew`, creates the
exact save directory and copies the two files with `CreateNew`. Source streams
deny write/delete sharing and are hashed before and after copying. Only a full
readback produces a separate `seed-stage-receipt.private.json`. A partial claim,
directory or file remains evidence and cannot be resumed or overwritten.

After staging, the independent normal launch plan and lease must each contain
the identical closed `seed` object: `{mode: LOAD_ONE_PRIVATE_DISPOSABLE_SEED,
manifestSha256, stagingReceiptSha256}`. This avoids circular stage/launch hashes.
`before-smoke` and every existing runner/controller `admit-launch` recheck the
source provenance, source and target census/hash/XML, current tooling pins,
claim/receipt linkage and absent log. Original and backup checks remain
unconditional. `after-smoke` preserves the game's resulting private save state.

The focused tests use generated isolated files and synthetic provenance, never
original saves or a real campaign. Public runner/controller route tests stop at
deliberate prelaunch boundaries. A separately planned real load is still needed
to qualify game compatibility and plugin attachment. Loading an older generated
campaign exercises `onGameLoad`; it does not establish new-game acceptance.
# Guided interaction watchdog

`run-native.ps1 -Phase launch -SessionWatchdogMs 7200000` selects a two-hour
automation safety watchdog only when both the current launch `plan.json` and
`native-lane-lease.private.json` declare the same integer `sessionWatchdogMs`.
The default remains 480000 milliseconds; the supported range is 480000 through
43200000. The controller derives its bound from this budget with the existing
10-second supervisor margin. Its actual metadata records the declared watchdog.
Expiry is `INCOMPLETE_AUTOMATION_WATCHDOG`, with the raw failed supervisor result
retained; it is not proof of a product failure.

Job ownership, explicit stop/reap, current process identity, save protection and
seed admission remain required. Preparation/staging source pins remain immutable
historical provenance; launch configuration must independently pin current code.
Guided loading does not admit the one-shot owned-node-kill operation: its separate
full source-row guard still refuses a changed controller against old stage pins.

