# Native shadow hook proposal (issue #11)

Checkpoint: 2026-10-07. This is an exact-source proposal, not an installed adapter or G1 acceptance. `NearFutureModPlugin` remains the compile-only stub. No game was launched, mod deployed, save opened/modified, live object captured or native actuator enabled. The next experiment must remain disabled by default and shadow-only after separate implementation approval.

## Inspected identities and reproducible evidence

The configured installation is `H:/Games/Starsector`; its observed runtime is Starsector 0.98a-RC8, bundled Azul Java 17.0.10+7 and Nexerelin 0.12.2c. Hashes are observations, not a statement that the installed Nex JAR was reproduced from the source commit.

| Inspected input, read in place | SHA-256 |
| --- | --- |
| `starsector-core/starfarer.api.jar` | `ca75796f0c31afef1dc22c664222657e1340ab3fa20faf10dcca96db4e307ce0` |
| `starsector-core/starfarer.api.zip` | `96ceb1c0bf987ce8a2ef5955afc412a122f35f0f7a294ea0a79ca937345cd041` |
| `mods/Nexerelin-0.12.2c/jars/ExerelinCore.jar` | `d7e1884e3c186480ad448b11a7aeac85ef503f17b5fd4d3686b8178550d01d18` |
| installed `data/config/exerelin/strategicAIConfig.json` | `408eb96b08966d3258ce356f8f676c3c2587ead0af01635fcf4f1aa275dc59f2` |

The legally reusable Nex source is pinned to [a669f4d0740e95a4acbb6b894dde09ade67aa754](https://github.com/Histidine91/Nexerelin/tree/a669f4d0740e95a4acbb6b894dde09ade67aa754), with its MIT notice retained by existing reference/boundary work. `DiplomacyManager.java` blob `9fa8774d6b9ad809345eb7af13fd0456e618e11a` has SHA-256 `49bb9856a873ae6963839acb65ae994a6adf10d426301743314df556002f3435`; `NexConfig.java` blob `2bfbcf847230233a7229a788a29b850dc0d9518f` has SHA-256 `49afe6cc0267302c471129e7a92b91a211150db04356b2b6ceb2409a91c07be0`. Bundled proprietary API source was read directly from the local ZIP; no API/game binary or proprietary source copy is included here.

Installed public signatures were inspected using pinned JDK 21.0.8 `javap`. A disposable signature-only Java class in `.tmp/native-hook-proposal` compiled with `javac --release 17 -Xlint:all -Werror`, referencing the installed API and Nex JARs in place. Its class major version is 61. It overrides only the four documented callbacks below and type-checks transient-script registration, relation/config reads and persistent-data access. It was never loaded, executed, packaged or registered. Compilation confirms symbol compatibility; it establishes neither callback thread behavior nor read purity in the game.

```powershell
& "$env:JAVA_HOME/bin/javap.exe" -classpath "$env:STARSECTOR_HOME/starsector-core/starfarer.api.jar" com.fs.starfarer.api.BaseModPlugin com.fs.starfarer.api.ModPlugin com.fs.starfarer.api.EveryFrameScript com.fs.starfarer.api.campaign.SectorAPI
& "$env:JAVA_HOME/bin/javap.exe" -classpath "$env:STARSECTOR_HOME/starsector-core/starfarer.api.jar;$env:STARSECTOR_HOME/mods/Nexerelin-0.12.2c/jars/ExerelinCore.jar" exerelin.campaign.DiplomacyManager exerelin.utilities.NexConfig com.fs.starfarer.api.campaign.FactionAPI
```

The disposable probe source SHA-256 is `cc99d21086fa9682e22d82486b8e6ac9652176cf0ece4772e07bd8caccfe8ce9`. Its source contains the documented no-op callback overrides and static signature-only methods above, with no initializer/main or registration. The observed compile command was:

```powershell
& 'G:/dev/Android/AndroidStudio/jbr/bin/javac.exe' --release 17 -Xlint:all -Werror -classpath 'H:/Games/Starsector/starsector-core/starfarer.api.jar;H:/Games/Starsector/mods/Nexerelin-0.12.2c/jars/ExerelinCore.jar' -d .tmp/native-hook-proposal/classes .tmp/native-hook-proposal/NativeSignatureProbe.java
& 'G:/dev/Android/AndroidStudio/jbr/bin/javap.exe' -verbose .tmp/native-hook-proposal/classes/nf/proposal/NativeSignatureProbe.class
```

The compile succeeded; `javap` reported major version 61. The disposable source/classes stay ignored and are not part of the mod artifact.

These commands assume the recorded Windows installation and pinned JDK; set the environment paths explicitly. POSIX classpath separators differ. The ZIP's exact `ModPlugin.java`, `EveryFrameScript.java`, `EveryFrameScriptWithCleanup.java` and `SectorAPI.java` entries were inspected without extraction. Relevant source paths and facts below are the evidence, not inferred engine internals.

## Confirmed callbacks and gaps

| Exact public signature | Bundled API documentation | Proposed finite handling | Unproved guarantee |
| --- | --- | --- | --- |
| `BaseModPlugin.onGameLoad(boolean)` | After a game loads; also after new-game plugins | Revoke the prior IPC fence and request a fresh diagnostic epoch; abandon all prior captures/results | No before-load callback; thread identity, atomicity and earlier worker/result quiescence are unobserved |
| `beforeGameSave()` | Before saving | Revoke IPC result acceptance immediately and request owner-side capture/projection invalidation; keep only the last complete immutable checkpoint | Save callback thread, asynchronous engine save timing, serialization ordering and coherent native fields are unobserved |
| `afterGameSave()` | After saving | Request a fresh epoch; resume only after the designated frame owner reconciles it | No assumption that this means all engine save work/threads have quiesced |
| `onGameSaveFailed()` | Saving failed | Keep the prior epoch revoked; request explicit fresh diagnostic initialization on the frame owner | No guessed rollback or retry of native manager mutations |
| `EveryFrameScript.advance(float)` | Elapsed seconds since the last campaign frame; campaign clock converts to days | Designate and check an exact thread object before reading any live fact; process one bounded diagnostic turn | No documented thread identity or stable rendered-frame counter; fast-forward/substep/render correspondence is unobserved |
| `EveryFrameScript.isDone()` / `runWhilePaused()` | Completion cleanup eligibility / whether paused advances occur | Return bounded state only; choose pause behavior explicitly in a future experiment | Neither method is an exit notification |
| `EveryFrameScriptWithCleanup.cleanup()` | An entity carrying the script is removed from the campaign engine | Not an exit or sector-transient shutdown substitute | No proof this is called for sector scripts, load replacement or application exit |

`ModPlugin` also documents that a plugin instance is created when the game starts and that `onEnabled(boolean)` precedes `onGameLoad` when relevant. Its interface has no public before-load or application-exit callback. `CoreLifecyclePluginImpl` extends `BaseModPlugin` and exposes the same relevant save/load surface; its name does not supply missing hooks. `Global.getCurrentState()` and `Global.getSector()` exist, but polling them is not a certified transition barrier.

`SectorAPI.addTransientScript(EveryFrameScript)` and `removeTransientScript(EveryFrameScript)` are exact installed signatures. The API exposes `getTransientScripts()` as well. Its Javadocs identify ModPlugin/transient CampaignPlugin implementations as examples outside a save when describing persistent-data storage; the inspected script registration method itself gives no serialization/thread guarantee. Native save inspection must verify that the selected transient script and its service graph are absent. Use the exact owned script instance for removal; do not remove arbitrary scripts by class or alter Nex managers.

## Facts available for a diagnostic copy

The narrow candidate subscribes to explicit, bounded faction pairs and the configured peace threshold. It can copy finite raw float32 bits from `FactionAPI.getRelationship(String)` and `NexConfig.minWarWearinessForPeace` after the exact owner-thread guard passes. Missing faction/unsupported value/configuration fails with a static unavailable result; there is no fabricated zero/default fact. This is signature/source support for a diagnostic read, not an observed purity, coherence or algorithm-equivalence certificate.

The first concern's weariness fact is currently unavailable to a passive adapter. The installed JAR exposes public static `DiplomacyManager.getWarWeariness(String)` and `(String, boolean)`, but the pinned source inserts a missing faction entry into the manager's protected weariness map. Missing manager also logs and returns zero. Adjusted weariness additionally queries enemy relationships. There is no public weariness-map accessor in the inspected installed class. Do not call this getter under a read-only label, invoke manager creation, use reflection, inject a same-package subclass, or treat a returned zero as evidence that the source fact existed. An approved upstream passive read/coverage seam or separately justified effect-bearing capture would be needed.

Consequently relations/config diagnostics do not produce a complete `NexWorld`, a WarWearinessConcern input or a G1-eligible certificate. Existing `CampaignCapture` and `ObservedRevisions` can certify only explicitly delivered cooperative revisions. Sampling values before/after a chunk cannot prove every intermediate writer or detect ABA changes. Every Nex/direct/extension writer, merged definition, inherited concern lifecycle and ownership change still needs the [boundary inventory](../nex-boundary/README.md) and actual coverage experiments.

## Proposed ownership composition, after approval

1. Keep the mod plugin and service references outside persistent campaign data. Prepare trusted configuration/content hashes, identity, OS entropy and any background executor/IPC connection outside the frame callback. A worker receives bounded immutable values only, never `SectorAPI`, faction objects, `CaptureSource`, mutable collections or managers. No file/SQL/socket wait/large encoding/crypto/worker join runs in a callback.
2. Save/load callbacks perform only finite fence/epoch requests. `nf.capture.CampaignCapture.invalidate()` is thread-guarded, so it must not be called from a callback whose ownership is unknown. Request invalidation through a bounded value flag; the designated frame owner invalidates capture/projection and rebuilds before polling any queued result. IPC `SessionFence.invalidate()` can immediately revoke acceptance. Any callback/frame-owner thread mismatch must fail closed and suspend diagnostics; it must not silently reassign ownership.
3. The first approved `advance` observation can establish an exact callback-thread token for the experiment. It still cannot prove that this is the engine's campaign thread or that every later lifecycle callback has the same owner. Instrument finite callback kind, invocation ordinal, same-owner boolean and copied context only; live thread/game objects remain local. Subsequent mismatches invalidate the epoch and return unavailable before live reads.
4. Reconcile lifecycle/context changes before reading or accepting results. Generate a fresh nonzero runtime session outside the frame callback; never revive the saved session. Content, ruleset, universe/history, provider ownership or observed revision drift discards queued work. Source sessions/ownership generations cannot wrap; exhaustion closes the owner until explicit fresh initialization.
5. Limit the diagnostic turn to eight bounded reads, at most 64 subscribed facts and 8192 logical bytes. The present capture API requires an actual monotonically increasing campaign frame number and shares its allowance across repeated calls for that frame. No inspected API supplies that number. A per-`advance` invocation counter is not an established rendered-frame counter, especially during fast-forward. Therefore do not claim the existing eight-per-frame or p99 <=2 ms guarantee until callback/frame correspondence is observed or a reviewed exact frame provider exists. No live capture should be wired by guessing a clock/frame source.
6. No native apply, manager suppression, concern generation/abort, relationship/weariness write or IPC strategic payload is enabled. Only a diagnostic immutable view may be considered after capture/lifecycle evidence. Current public wire/probe schema cannot carry these private facts; registering new capability/payload schemas with Rust/Java parity precedes any IPC transfer.

A missing before-load/exit barrier means this design is incomplete for whole #10/#11 acceptance. An observer cannot honestly promise invalidation before those transitions with the inspected surface. Do not substitute a shutdown hook, raw game-state polling, reflection, timer or entity-cleanup callback without an exact reviewed contract. Abnormal process loss must remain fail-closed through revoked/new sessions and owner-private stale rendezvous handling; no native game process is controlled by this proposal.

## Minimal save binding, not a service graph

`SectorAPI.getPersistentData()` is an exact `Map<String,Object>` API documented for keeping mod data across sessions. A future owner-approved binding may occupy one mod-specific namespaced key and contain one bounded immutable ASCII String carrying a versioned ordered record: universe/history IDs, campaign/branch IDs if assigned by trusted identity policy, recognized content/ruleset hashes and known committed frontier/version values. Validate exact field count, widths, encoding, supported version and trusted scope on load; unknown/missing/contradictory bindings remain unavailable. Use a strict documented parser and a small hard byte ceiling (proposed 1024 bytes); avoid custom serialized records, immutable-collection implementation classes or XStream registration. Allocate/check outside the callback and publish one complete String, not partial updates.

The binding must not contain account/device keys, tokens, runtime sessions, live game objects, workers, channels, queues, clocks or service references. Do not derive identity from object addresses, names or save paths. A new or unbound campaign must wait for explicit trusted binding preparation; the frame callback must not call native `genUID`, read files, hash content or generate credentials to fill the gap. No binding is written by the current stub or this proposal.

Save-time data must be only the last complete validated binding/shadow checkpoint. Staged transitions remain invisible. A callback's name alone is insufficient proof of save safety: actual save inspection/reload/failure recovery and ordering must confirm this value is serialized without the plugin/service graph. There is no live multi-field native pointer swap or acknowledged strategic commit in this slice.

## Required next evidence

Before production wiring, approve a narrowly disabled diagnostic adapter experiment and exact source/runtime dependencies. Observe callback threads/order and multiple callbacks per rendered frame under paused, normal/fast speed, combat/title/load/save/failure transitions. Establish an actual frame provider or narrow the performance claim. Inspect a disposable save before/after to prove the minimal binding and absence of secrets/services; no private save or log belongs in public evidence.

Then establish a passive weariness read and complete mutation/extension inventory with source/JAR equivalence where required. Observe mixed-chunk/ABA mutation, content/provider/session/frontier drift, stale queued results, save/reload/repeated publication and atomic native apply if ever proposed. Matched-save frame tails/allocation/GC and dead/slow/saturated-node responsiveness remain required by [performance methodology](../performance/README.md). Until these gates pass, thread/lifecycle/capture G1 and native authority remain UNOBSERVED/UNAVAILABLE; relations/config copied facts remain diagnostic only.

The [AutoColonyManager debugging notes](../starsector-debugging.md) explain the installed script-classloader reflection hazard and native UI/logging pitfalls. Java17 signature compatibility and headless pure tests cannot establish in-game class loading. Native UI is unavailable in this session, so no live hook, save or latency result is inferred.
