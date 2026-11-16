# Optional authenticated probe adapter

Refs #99 and #139. This source package is an opt-in diagnostic companion, with
no game authority or social/UI implementation. The ordinary
`nf.adapter.NearFutureModPlugin` is unchanged and has no link to the companion,
process, socket, crypto or JFR classes. Do not enable the companion in an ordinary
play installation until the named native profile is qualified.

`java/probe` is JDK-only, targeting Java17 with the repository's pinned JDK21.0.8
compiler. It implements the separate neutral module v1 protocol with the final
32-byte generation ACK: `NFMBND01`, service session16 and generation u64BE.
The host protocol is owned by #139 (`docs/modules/probe-v1.md` in its branch).
The old protobuf/local authentication and signed/canonical formats are unchanged.

The companion starts one daemon owner at application load only when
`nf.probe.enabled=true`. Its worker reads the explicit absolute
`nf.probe.executable` and hex SHA256 `nf.probe.sha256`, verifies at most64MiB,
and starts that executable with the single argument `serve-parent`. These
properties contain no authentication secret. The worker generates a fresh32-byte
secret and sends it only through the actual child's anonymous stdin pipe.
Readiness comes from that child's stdout; HMAC binds endpoint, nonces,
negotiated limits, fresh service session, request ID, sequence and generation.
No persistent discovery/adoption or credentials in arguments, environment,
logs or files are supported.

Campaign callbacks allocate/copy only fixed-size values and exchange at most one
pending32-byte challenge and one reply. A transient script runs while paused,
polls at most one result per frame, and submits at most8 requests per binding,
120 callback frames apart. Any unavailable/error result stops further requests
until a new binding. Five seconds bounds queue/startup/handshake/application
work; negotiation is additionally bounded to2 seconds and the application
round trip to the requested timeout (at most1 second). There are no per-request
automatic retries. File/hash/crypto/process/pipe/socket work and cleanup waits
stay on the worker.

Before save, the callback synchronously revokes its mailbox and removes the
transient script. After save/failure and on game load it creates a strictly newer
binding. Revocation also cancels a request already on the worker. No old mailbox
can deliver to a new one. The host applies a new binding and invalidates old
connections before acknowledging it; Java verifies the ACK session/generation
before reconnecting. The application service may survive campaign replacement;
its finite child lease lasts at most60 seconds and allows8 rebindings. A later
request can create a fresh verified service after expiry. A partial/failed bind
discards the uncertain owned service. Cleanup failure disables that owner.

All plugin fields holding runtime references are transient; the script is added
only with `SectorAPI.addTransientScript`. No persistentData, XStream converter,
save schema, token, socket, process handle or worker is registered for saving.
This is source structure; actual native save exclusion is still unproved.
The companion exposes a closed `diagnosticStatus()` string. For saved native
observations, explicitly set `nf.probe.evidence` to a fresh absolute private
JSONL path. Only the worker opens/formats/writes/flushes/closes it, using
CREATE_NEW; an existing original refuses. It records at most256 fixed-schema
facts (less than128KiB), with one nonblocking callback slot and drop accounting.
Facts contain public request/session IDs, generation, closed status, worker echo
verification and actual callback-consumed replies. They contain no token, payload,
peer text, widget or path. Correlate WORKER_RESULT and CALLBACK_CONSUMED with the
same IDs/session/generation; a handshake or worker result alone is insufficient.
`callbackNanos` measures sampled advance work up to the evidence handoff; it
excludes bounded fact allocation/publication. CLOSED counts are explicitly a
snapshot and may omit concurrent shutdown publications; missing/dropped facts
are UNKNOWN. CLOSED can be absent on JVM exit. Evidence failure becomes
unavailable without preventing transport cleanup or terminal worker status.
The driver must inspect/hash saved evidence alongside process receipts and
original PNGs. This adds no renderer, console command, reflection or callback I/O.

Closing a request connection detaches it without stopping the service. Worker
shutdown closes the parent's pipe and reaps only its own child, separately
reporting forced cleanup. On JVM exit the anonymous parent pipe closes and the
sidecar revokes on EOF; actual ordinary game-exit behavior still needs native
observation. A daemon alone is not proof of game cleanup.

## Local builds and source checks

```powershell
.\java\gradlew.bat -p java :probe:check --offline --no-daemon --max-workers=2
.\java\gradlew.bat -p java :adapter:probePackage -PgameDir=<licensed-installation> --offline --no-daemon --max-workers=2
.\java\gradlew.bat -p java :probe:probeProcessTest -PprobeExecutable=<built-nf-sidecar> -PprobeSha256=<sha256> --offline --no-daemon --max-workers=2
```

`probePackage` stages only under `java/adapter/build/probe-package`; it never
deploys, launches a game or copies proprietary libraries. It contains the two
versioned Java JARs, explicit companion mod metadata and generated artifact
hashes/compatibility metadata. The Rust binary is a separate pinned #139
artifact. Native staging must pair the exact ACK32 host with this package;
the earlier16-byte-ACK artifact is incompatible. The accepted root headless
profile remains build-identity only.

`syntheticCheck` includes lifecycle and hostile-wire checks. The explicit
process test uses real Java-parent/Rust-child pipes/TCP, verifies application
echo, ACK/rebind, stale rejection, fresh sessions, cancellation, executable
pin refusal and owned cleanup. It does not launch Starsector or qualify the
game loader, campaign callbacks, performance, saves or mod coexistence.

## Native readiness — 16 November 2026

The inspected installation identifies Starsector0.98a-RC8 and Zulu17.0.10+7-LTS
Windows x86_64, with ToT1.0.11 and Nexerelin0.12.2c in the enabled profile.
Core `starfarer.api.jar` exactly matches ToT's bundled0.98a patched API:
SHA256 `ca75796f0c31afef1dc22c664222657e1340ab3fa20faf10dcca96db4e307ce0`.
ToT's mod JAR is
`cffbca3b9d33a17dec2ebe36005a1273ff908411b5d2a0c24cf3567b88e92f5c`.
The API source ZIP is separately pinned as
`96ceb1c0bf987ce8a2ef5955afc412a122f35f0f7a294ea0a79ca937345cd041`;
source/binary reproducibility is not claimed. Licensed public-API compilation
passed for the small used surface; it does not prove loader permission.

The [ToT author's installation instructions](https://www.ashesofthedomain.info/module/TheoryOfToolbox)
require replacing the core API with its supplied patched JAR. No alternate API
is bundled by this package. Installed-JAR inspection found a full API dependency
under an unselected mod's development libraries and ToT's patch installation
input; neither is in the enabled declared mod-JAR set. One enabled mod declares
two combat API overrides; neither is a probe lifecycle interface. This is a
declared-path inventory, not live class provenance or economy compatibility.

**Native status: not ready.** Existing original launcher files disable bytecode
verification. They must not be reused as evidence of a permitted verified run,
and originals were not changed. Original global autosave is enabled/default-on
with3 slots;178 original save files were hashed but no fresh backup/disposable
fixture was qualified. No game launch, input or deployment occurred.

Recommended first placement is this explicitly enabled, game-owned background
child attempt, after a verification-enabled isolated launcher and protected
fixture are qualified. Exact loader permission is unknown; no blanket prohibition
of ProcessBuilder has been demonstrated. If the loader denies the route, retain
the phase/class failure and use a supported external owner only after defining
a real in-memory game credential handoff. Current parent pipes cannot be adopted
by unrelated processes. Do not bypass the loader, disable verification, put a
second API JAR on the core/mod classpath, or run ToT without its required patch.

The finite native plan remains prepared and unexecuted. Running/paused replies,
combat/return, save/reload or switch, stale rejection, lost-host responsiveness,
save exclusion and ordinary game exit all require original native evidence.
