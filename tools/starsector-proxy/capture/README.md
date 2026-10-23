# Direct capture persistence

This maintained source saves complete raw MCP results and their exact original PNG
before the MCP reply is returned. It uses the existing private caller intent,
slot, ownership witness, root and batch clocks. It grants no game-input authority.
Freshness and test success remain separate from successful file persistence.

Run `capture-stdio.mjs <configuration-path> <configuration-sha256>` through the
owned SDK client and Windows Job supervisor. The loaded Codex connector is a
separate process: creating this executable does not hot-swap that connection.
An owned SDK client can use this entry point without restarting Codex.

`build-runtime.mjs` checks the pinned installed 7.1.0 package and creates a
generated cache below the ignored computer-use dependency's `node_modules`.
Only the previously qualified awaited-result hook and stdio embedding export
change. The generated server and declarations must match their qualified hashes.
The builder verifies existing cache contents; it never overwrites drift or a
partial cache. Installed dependency bytes remain untouched. This qualification
currently supports Windows x64 with the default pinned native addon; an explicit
native override is refused.

`vendor-pins.json` contains public relative paths, lengths and SHA-256 values.
`vendor-patch.json` records the exact source edits. Generated vendor/native bytes
are dependencies, not repository source. Configuration pins are checked before
session creation, but static module imports run earlier; this is not a claim of
pre-import attestation of the whole dependency graph.

Private plans, root leases, configurations, intent pointers, original images,
logs and receipts belong under ignored `.tmp/starsector-proxy`. The image in
`fixtures/readiness.synthetic.png` is a synthetic test vector, with no desktop,
game or private content. Test output is generated only below ignored `.tmp`.

The relocated suite retains the exact historical projection qualification against
12 batches and 65 frames. Supply `NF_CAPTURE_PROJECTION_FIXTURE_ROOT` pointing to
the pinned private inputs when running that test. Supply
`NF_CAPTURE_TEST_POWERSHELL` as the qualified PowerShell executable for the
composition integration test. The remaining tests use synthetic boundary data.
The relocation passed 64 tests and affected JavaScript type checks under an owned
Windows Job. This does not qualify a live capture or campaign.

`probe-stdio.mjs <input-path> <input-sha256>` is a separate catalog, policy and
window-list diagnostic. Its private input names the Node executable, this
entry point, configuration hash, environment, workspace and receipt path.
Run it under the existing bounded Windows Job. It sends no game input and does
not call `doctor`, capture screenshots or access the clipboard. It awaits the
SDK's close and checks its owned server PID is absent. A successful probe proves
the connection and cleanup only.

`single-call.mjs` executes one caller-authorized observation or capture and retains
its raw result, terminal status and child cleanup. Its optional
`refreshOwnedWitnessAfterConnect` request requires the same explicit option in
the root-admitted configuration. After connecting, it runs a fixed, hash-pinned
PowerShell reader against that configuration's exact current producer file,
retains the real child arguments/status/output, verifies the original controller
and game process identities, and reads the same HWND through the connected SDK.
Only then does it freeze and publish the capture intent. The initial caller
request remains preserved; no tool return is fabricated for the child read.
The sample timestamp supplies the unchanged five-second freshness limit, checked
again before capture. Root and batch clocks, targets and input authority remain
unchanged. Internal window observations count against the scenario observation
cap. Closed failure codes, phase and measured age remain in the private receipt.
This optional path passed focused synthetic boundary checks; its live game
qualification is pending.
# Guided session capture

An admitted `rootSessionWatchdogMs` may select the same explicit launch watchdog
(480000 through 43200000 milliseconds). Extended sessions require matching
`sessionWatchdogMs` in the caller root clock and producer witness, the exact
admitted runner command, and the actual producer/effective deadlines. Omission
keeps the legacy 480000 contract. This changes automation lifetime only: each
capture still has its 180-second batch and 90/60-second capture/persistence gates;
the single-call caller retains its 20-second tool bound and five-second witness
freshness. Existing frame custody, sticky refusal and screenshot-only SDK policy
are unchanged. No historical result becomes a success through a new watchdog.

