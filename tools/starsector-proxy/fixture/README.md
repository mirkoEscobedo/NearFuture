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

The ten-field binding above is unchanged. Without explicit seed authority,
preparation and launch require an empty private save root. The generation
profile's post-Continue observation exception is described below. A seed claim or receipt also refuses an unseeded
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

# Immutable seeded campaign entry and exit

The schema3 `DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE` input profile uses the existing
schema1 `CAMPAIGN_BINDING_ONLY` protected launch with one staged disposable seed.
Its original plan and lease both declare
`campaignControl: {schemaVersion: 1, profile: "DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE"}`
and the same absolute integer `callerAdmissionDeadlineMs`. The original plan binds
the prelaunch `historyBaseline` and exactly B1/E1/E2. Bounds remain eight actions,
three providers and 480000ms. If an explicit `sessionWatchdogMs` is present, both
documents must declare 480000. Current critical tooling and the complete provider
runtime must be pinned independently of historical seed-staging pins.

After actual launch, freeze `campaign-postlaunch.private.json` with exactly:
`schemaVersion: 1`, `kind: "FINITE_CAMPAIGN_LAUNCH_PREDECESSOR"`, hash references
`plan`, `lease`, `controller`, and original `rootClock`, `producerWitness`,
`rootCallRecord`, `historyBaseline`, `maxReservedActions`. Each reference is
`{path, sha256}` and names the original canonical launch files. The controller's
actual birth/kernel identity, watchdog and recorded controller/outer deadlines
must agree with the witness. The original caller cap and conservative minimum
remain authoritative. Do not overwrite launch files with postlaunch observations.

Each new batch uses `<batch>-plan.private.json`, `<batch>-lease.private.json`,
`<batch>-authorization.private.json` and `driver-<batch>-owned.private.json`.
The authorization uses schema3 and binds those exact paths and digests. Plan and
lease share the closed campaign object: `flow`, `windowId`, `rootClock`,
`producerWitness`, `rootCallRecord`, `historyBaseline`, `maxReservedActions`, and
`launchPredecessor: {path, sha256}`. The absolute action maximum is the original
baseline plus eight, including B1. Consent remains the existing matching
`directHostConsent` grant. Schema2 retains canonical filenames and refuses this
predecessor extension.

B1 permits optional activation then Play. Its successful provider closure and
`B1-menu-inspected.private.json` bind `B1-after/F1.original.png` and the matching
`capture-001` raw/metadata files. The inspection identifies `STARSECTOR_MAIN_MENU`
and its exact Load Game coordinate. E1's matching `b1MenuReceipt` reference in plan
and lease admits the current game HWND only through the original owned process
lineage; a new or reused PID is insufficient.

E1 permits exactly three left clicks followed by one exact Escape. The first click
uses the inspected Load Game coordinate; the driver verifies the sole staged row
and Load button on current originals. Known action results require complete
original AFTER images and renewed focus/window checks before the next action.
The driver saves `E1-seed-paused/F1.original.png` and matching `capture-001` raw
and metadata, then freezes `E1-seed-paused-inspected.private.json`. Its closed
`STARSECTOR_SEED_PAUSE_INSPECTION` record binds root/controller/source/claim/run,
the last Load reservation, original seed manifest/stage digests, frame digests and
chronology, and the driver's `INTENDED_DISPOSABLE_SEED_PAUSED` inspection. Retain
actual transport call/attempt IDs and timestamps when constructing frame provenance.
Fresh focus checks follow that capture. A receipt alone leaves frame freshness
`UNKNOWN`; it cannot replace visual inspection or establish campaign compatibility.

After known E1 closure, the existing `E1-menu-inspected.private.json` shape binds
the ordinary menu original under `E1-after/`. E2's matching `e1MenuReceipt` grant
permits the inspected Exit coordinate. It rechecks original B1/E1 phase files,
claims, known closure and paused-image custody, including after E2 claims ownership.
Old provider clocks remain historical evidence, not renewed live deadlines.

For one optional confirmation, both E2 documents must additionally freeze exactly
`exitConfirmation: {schemaVersion: 1, dialogText: "Exit game?", confirmLabel: "Yes"}`.
After Exit, the driver saves the current original under `E2-exit-confirmation/`
and freezes `E2-exit-confirmation-inspected.private.json`. This uses the same
root/source/claim/reservation/seed/frame custody fields as the pause record, kind
`STARSECTOR_EXIT_CONFIRMATION_INSPECTION`, and inspection fields `issuer`, `driver`,
`inspectedAtMs`, `state: "EXIT_GAME_CONFIRMATION"`, `dialogText`, `confirmLabel`,
`confirmCoordinate`. Only the matching declared dialog/button and exact inspected
coordinate admit the second click. Missing or unexpected confirmation stops input.

Claims and reservations are never refunded. Keep original failures, cumulative
history, saved originals and owned cleanup evidence. This route cannot generate
a seed, use personal saves, start New Game, save/reload, kill the sidecar or certify
lifecycle/performance acceptance. The existing seed provenance/staging contract
remains required; actual game validation and loaded frontend qualification remain
separate from generated-fixture and stdio/HTTP checks.

# First New Game page inspection

The separate schema4 `NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE` profile admits
an empty protected fixture only. Original plan and lease both declare
`campaignControl: {schemaVersion: 2, profile: "NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE"}`,
the original caller deadline and history, and exactly B1/E1. The ceilings remain
480000ms, eight cumulative actions and three providers; the unused provider slot
does not authorize another phase. Seed authority, seed manifest/staging metadata
and every private save entry are refused at launch and finite admission.

Use the same immutable original launch/predecessor, phase-local files, source
pins, process lineage, exclusive claims, clipped clocks and finite consent.
B1 permits optional activation and Play. After known B1 closure, the driver
inspects its saved full original main-menu image and freezes the existing
`B1-menu-inspected.private.json` shape with the new flow and the exact inspection
field `newGameCoordinate` instead of `loadCoordinate`. The original PNG, raw result,
metadata, source, reservation and capture-completion chronology remain required.
When republishing an original through `persistDirectFrame`, retain actual call and
attempt provenance; automatic capture receipt alone does not supply that proof.

E1 permits exactly one left click at that inspected New Game coordinate, then
observations only. Capture and inspect the first wizard page. E2, cancellation,
wizard advancement, typing, keys, saving and a second click are unavailable in
this initial profile. It therefore ends with maintained identity-bound owned
cleanup and after-smoke protection. Keep its final action reservation and failed
controller stop outcome as evidence; do not clear, refund or call forced cleanup
ordinary exit. A later cancellation route requires its own observed controls and
qualified finite implementation.

Report first-page observation, provider closure, forced cleanup and original-save
protection separately. This profile creates no saved seed and qualifies no
campaign compatibility, lifecycle authentication or alpha capability. Generated
tests do not establish the loaded frontend or real wizard state.

# Generated-name and first Continue inspection

Schema5 `GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE` is a separate profile.
Both original launch documents declare exactly
`campaignControl: {schemaVersion: 3, profile: "GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE"}`
and exactly B1/E1/E2. It requires fixture binding3 and its only additional field
`fixtureProfile: "VANILLA_UNCOMPRESSED_GENERATION"`; this binding cannot carry a
recording-profile digest. The fixture retains legacy recording, all installed
runtime pins and private vault initialization. Preparation enables zero mods
and requires the source compression setting already false; it never rewrites
compression or original files. Protection rechecks the exact empty enabled-mods
recipe and literal uncompressed setting even if receipt digests are refreshed.
Validation accepts the game's float suffixes such as `1f`, leading decimal
points and semicolon separators in its parser view only; preparation preserves
those bytes and strings.
Legacy fixture bindings and input profiles cannot select this recipe.

Original and phase plan/lease pairs also bind `absoluteMaxReservedActions` and
the identical closed `historyAdmission:{path,sha256}` descriptor. Its path is
the scenario-local `root-history-baseline.admitted.private.json`; its hash binds
the root-admitted carried history. That receipt declares `schemaVersion:1`,
`kind:"ROOT_ADMITTED_CARRIED_PRELAUNCH_HISTORY"`, `issuer:"/root"`,
`driver:"/root/recovery_game_driver"`, the identical `historyBaseline` and
`maxReservedActions`. The campaign/predecessor ceiling must equal that admitted
absolute limit, between the already reserved count and at most eight more.
Reusing a run never replenishes the carried limit. Root admission must preserve
the historical ceiling; a new self-consistent phase cannot raise it. Other input
profiles retain their existing budget contracts.

B1 uses the existing inspected main-menu receipt with `newGameCoordinate`. E1
first sends New Game, then freezes `E1-generate-inspected.private.json`: schema1,
kind `STARSECTOR_NEW_GAME_GENERATE_INSPECTION`, `scenario`, `rootRecordKey`,
`controllerSha256`, `sourceSha256`, `claimSha256`, `runID`, `reservationID`,
`frame`, `inspection`. Its original/raw/metadata files live in `E1-generate/` with
the standard `F1.original.png` and `capture-001` names. The inspection is exactly
`issuer`, `driver`, `inspectedAtMs`, `state: "NEW_GAME_IDENT_CHIP_BLANK_NAME"`,
`nameBlank:true`, `generateLabel:"GENERATE"`, `generateCoordinate`. Fresh current
original bytes and post-capture focus admit only that Generate coordinate.

After Generate, save the original/raw/metadata under `E1-generated-name/` and
freeze `E1-generated-name-inspected.private.json`. Its fields are `schemaVersion:1`,
`kind:"STARSECTOR_E1_GENERATED_NAME_INSPECTION"`, `scenario`, `flow`, `rootRecordKey`,
`controllerSha256`, `e1`, `frame`, `inspection`, `provider`. The `e1` object binds
`sourceSha256`, `claimSha256`, `runID`, `newGameReservationID` and
`generateReservationID`. The inspection is exactly `issuer`, `driver`,
`inspectedAtMs`, `state:"NEW_GAME_IDENT_CHIP_GENERATED_NAME"`, `generatedName`
(nonblank, at most128 characters, no control characters), `continueLabel:"1. Continue"`
and `continueCoordinate`. The frame and known-closed provider shapes match B1.

E2 plan and lease bind that receipt through `e1GeneratedNameReceipt:{path,sha256}`.
Admission revalidates both E1 reservations, the Generate inspection, original
PNG/raw/metadata, historical B1/E1 authority and actual provider closure. E2
permits exactly one first Continue at its inspected coordinate, then observations
only. It uses no ordinary-menu/Exit receipt. Finish through maintained owned
cleanup and after-smoke protection, preserving the unresolved final reservation
and controller outcome. The first Continue destination is not assumed; no later
wizard, typing, key, Save, cancellation, seed provenance or lifecycle acceptance
is supplied. All prior failures, absolute action history and original deadlines
remain in force. A qualified source checkout and actually refreshed supported
frontend are required before live use.

Prelaunch, B1 and E1 still require empty private saves. Only after the exact known
E2 Continue may observations accept one ordinary `common` directory containing
only `core_shared_settings.json.data`, with these exact 41 UTF-8 bytes:
`{"campaignHelpPopupsOptionChecked": true}`. No BOM or newline is included.
Admission still checks current owner/claim/source, inspected coordinate, original
clocks and the known `AWAITING_AFTER` reservation. Links, extra files/directories,
different bytes, uncertain or incomplete actions and every further input refuse.
This UI preference is preserved output; it supplies no campaign save or seed.

# Disposable initial seed creation

Schema6 `DISPOSABLE_SEED_CREATION` is distinct from the preceding observation
profiles. Original launch plan and lease select exactly
`campaignControl:{schemaVersion:4,profile:"DISPOSABLE_SEED_CREATION"}` and binding3
`VANILLA_UNCOMPRESSED_GENERATION`. Both original and phase pairs bind identical
`seedCreation:{mode:"CREATE_ONE_PRIVATE_DISPOSABLE_SEED",preference:{path,sha256}}`
and `historyAdmission:{path,sha256}` descriptors. The fixed preference file is
`seed-creation-preference.private.json`; its closed fields are schemaVersion1,
kind `NATIVE_SEED_SHARED_PREFERENCE_V1`, view `Registry64`, node
`/com/fs/starfarer`, valueName `canSkipTutorial`, registryValueName
`can/Skip/Tutorial`, valueKind `String`, value `true`. Only this existing exact
HKCU Java preference is read. Native launch and protection recheck it without
enumeration, creation or writes.

The fixed `root-history-baseline.admitted.private.json` grant uses schemaVersion2,
kind `ROOT_ADMITTED_SEED_CREATION_HISTORY`, issuer `/root`, driver
`/root/recovery_game_driver`, identical `historyBaseline`, `maxReservedActions`,
and `previous:{result:{path,sha256},ledger:{path,sha256}}`. Both previous artifacts
must be actual terminal evidence with unchanged guard counters and the truthful
next attempt context. The cap is greater than the carried reserved count and at
most eight more. New authority does not erase prior failures or replenish input
within a run. Root-issued authority JSON uses canonical scalars and permits
insignificant whitespace; duplicate fields refuse. Legacy grants are unchanged.

The only route is eight left clicks: B1 Play; E1 New Game, Generate, first
Continue; E2 Explorer/Apogee faster start, Normal, Skip it, Start game. No
activation, key, typing, manual Save or extra provider is available. All original
root/provider/startup/call/action/cleanup limits remain. An unavailable preferred
choice stops the route instead of selecting another branch.

Each E1 control after New Game and every E2 control uses
`<batch>-wizard-<ordinal>-inspected.private.json`, kind
`STARSECTOR_SEED_WIZARD_CONTROL`, with the closed lineage fields in
`seed-wizard-proof.mjs`. Its fixed directory contains the original PNG, raw MCP
result and metadata. Inspection binds issuer, driver, time, state, exact label,
enabled=true, coordinate and generated name. E1 ends with
`E1-occupation-inspected.private.json`, kind `STARSECTOR_E1_OCCUPATION_INSPECTION`,
binding all three reservations and actual provider closure. E2 binds that receipt
as `e1WizardReceipt:{path,sha256}`. Every control is checked against the actual
clicked BEFORE, archived AFTER where present and inspection before dispatch.

The final original lives under `E2-created-seed/`; the closed
`E2-created-seed-inspected.private.json` has kind `STARSECTOR_NEW_SEED_INSPECTION`
and inspection state `NEW_GAME_CAMPAIGN_VISIBLE`. Its frame time, paths, bytes
and source capture must agree, followed by actual terminal E2 provider proof.
Observations after the known Start may tolerate the game's partial initial-save
write; partial files are never accepted as a usable seed.

After all owned handles are terminal and reaped, freeze
`seed-creation-closeout.private.json`, kind
`ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT`, with carried/actual counters, separate
raw cleanup outcome, original protection, exact two-file census and fixed
evidence hashes. The V2 manifest kind is `NATIVE99_DISPOSABLE_SEED_MANIFEST_V2`.
It adds canonical `sourceWorkspace` and uses immediate private scenario children;
its five provenance pins include that completed closeout. This forms an acyclic
chain: closeout pins XML/evidence, manifest pins closeout, stage authority pins
manifest. The consumer rereads actual acknowledged clicks, original-image and
provider custody, zero live source processes, original182/save178 protection,
the exact41-byte common preference, and ordinary unlinked XML files. Version,
uncompressed format, player-fleet reference, zero mods and 16 MiB total are
independently validated. Current complete tooling pins allow at most1024 unique
rows and262144 bytes for V2; a selected subset refuses. V1 retains its original
131072-byte inventory limit and provenance reader. Valid V2 staging composes with
the existing immutable seeded-entry launch validators; it grants no new input.

Tests generate OS, visual and cleanup boundaries. Their real transport/validator
coverage is source evidence, never a loaded-frontend or native campaign pass.

