# Starsector through OpenCode

## Reproducing the maintained finite route

Refs #130 and #92. The finite frontend, fixture, capture and cleanup sources are
selected from the preserved local rig; the landed PNG validator and its refusal
tests remain authoritative. Source checks qualify tooling only. Actual loaded
host identity, disposable entry/pause/observation and ordinary exit are separate
required native evidence; node authentication and performance remain #99/#100.

Start from a clean checkout with root vendored tools installed as documented in
`docs/delivery/integration.md`. Install the separate pinned libraries with the
command below. Windows x64, a signed-in desktop, qualified Node/PowerShell,
licensed game installation, candidate runtime artifacts and presentation agent
are declared external inputs. Existing OpenCode/provider configuration is needed
only for the OpenCode adapter. No credentials or personal saves belong in source.

`tools/starsector-proxy/finite-mcp.example.toml` shows the finite frontend's local
configuration fields. Replace its placeholders with qualified absolute paths.
Load/reconnect it through the supported client route, then retain the actual
catalog/schema and source/runtime identity before a launch. Updating source or
passing the synthetic catalog test does not refresh an already loaded frontend.
If no supported refresh is exposed, ask the host owner to reconnect/restart that
finite frontend against the qualified checkout; release the native lane meanwhile.

Use `fixture/README.md` for the hash-bound preparation/ownership/launch inputs.
The current qualified fixture shape is deliberately fixed; this is not arbitrary
installation/save-root discovery. A fresh frozen `plan.json` must additionally
bind the existing scenario-local fixture file before any provider is spawned:

```powershell
$plan.fixtureBinding = @{
    path = Join-Path $scenario 'fixture-binding.private.json'
    sha256 = (Get-FileHash -LiteralPath (Join-Path $scenario 'fixture-binding.private.json') -Algorithm SHA256).Hash
}
```

Freeze the completed plan before deriving its lease/authorization hashes.
The binding must identify that scenario and the admitted driver; another fixture
or changed binding refuses. Owner and child independently reread it, including
after the ownership probe. The probe passes its path/hash to the maintained
fixture protection/controller scripts; no ignored recovery script is required.
Source pins cover `fixture-binding.mjs`, the ownership helper and the transitive
fixture input, seed, snapshot and watchdog closure. Existing consent, single-use
claims, action counts, cleanup and deadline limits remain unchanged.

The required public-build matrix runs the portable adapter/parser/configuration
tests on Windows and Linux. Its Windows cell additionally materializes the pinned
capture runtime once, then runs fixture/capture checks with absolute
`NF_TEST_POWERSHELL` and `NF_CAPTURE_TEST_POWERSHELL` inputs. Those checks use
synthetic boundary data and launch no game. The historical projection test stays
an explicit private-input check via `NF_CAPTURE_PROJECTION_FIXTURE_ROOT`; a missing
projection input is not a public/native acceptance pass. The headless profile
still enables build identity only.

This setup adapts AutoColonyManager's locally working OpenCode agent and pinned
`@zavora-ai/computer-use-mcp` 7.1.0. OpenCode runs on the signed-in Windows
desktop; it does not use Codex's `@oai/sky` runtime. A Windows Job Object bounds
the proxy process tree without isolating it from that interactive desktop.
Do not run desktop checks from an isolated sandbox, a service session or WSL.

Install the separate pinned MCP dependency with
`npm ci --prefix tools/computer-use-mcp --ignore-scripts --bin-links=false --no-audit --no-fund`.
The driver imports explicit module paths and never uses npm's command wrappers;
disabling those wrappers preserves ordinary-file source custody on Unix as well
as Windows. Keep the root dependency files unchanged. The
existing `opencode` CLI and its already configured provider are required; no
credentials belong in this repository. The agent uses the same
`opencode-go/glm-5.3-flash` model as the local AutoColonyManager setup by default.
An explicit caller-authorized provider/model change can use `-Model provider/model`.
Confirm the installed CLI's model ID and required image/tool capabilities before
launching a game. Keep credentials in the local client or secret store; do not
include them in prompts, repository files, saved plans or command arguments.

The qualified local `unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL`
can opt into `-DisableThinking` and, independently, `-DisableParallelToolCalls`.
These supply the supported `enable_thinking: false` and
`parallel_tool_calls: false` request fields for this invocation. Omitted controls
leave their fields inherited. Both controls require a new run of this exact
local model and reject historical exports before effects. Requested-body
metadata records the selected combination; a requested field alone does not
prove delivery, backend compliance or serial execution. The batch guard and
actual request/action evidence remain required. This leaves the default
and global configuration unchanged and rejects other models or historical
exports before effects. `frames.json` records the explicitly requested body.
Synthetic checks qualify transport; real scenario assertions and deadlines
still apply.

`-CheckMcpOnly` performs only owned core MCP registration and strict connection
admission, then closes the owned service. It dispatches no model, agent, desktop
tool, image capture or game launch. It cannot combine with historical export.
Its `MCP_READY` receipt records the admitted repository directory and qualifies
connection only. The adapter resolves that directory before registration so a
terminal path separator does not create a separate OpenCode location from the
CLI's working directory. Rejections retain bounded,
sanitized response shape and server status in the existing private supervisor
evidence; no connection or game success follows from a generic error message.

From the NearFuture root, use PowerShell 7 on the signed-in desktop:

```powershell
pwsh -NoProfile -File tools/starsector-proxy/run.ps1
```

The runner discovers the installed Windows OpenCode executable (or accepts
`-OpenCodeExe`), defaults to `starsector-observer`, and owns OpenCode and its MCP
descendants through the native `workspace-template verify` supervisor with a
three-minute bound. It creates a separate verification root under
`.tmp/starsector-proxy`; the project's verification commands stay unchanged.
Do not attach to another person's OpenCode session. Serialize proxy calls;
do not run two game drivers concurrently.

New invocations use one private foreground OpenCode service for MCP registration,
model execution and export. It binds only loopback on an assigned port, uses new
in-memory authentication and an in-memory database, and exits through its owned
stdin lifetime. The existing Windows Job remains the process-tree supervisor.
Only the pinned core `computer_use` server is enabled, with direct tools
(`codemode: false`) and the existing 20-second MCP limits. Admission requires
exactly one connected `computer_use` row and the three configured companions
(`chrome-devtools`, `debugMcp`, `dart`) disabled. Valid partial or pending
profiles wait within that same bound; unknown, duplicate, malformed or active
companion rows refuse admission. The generic execution
wrapper is denied. Historical `-ExportSession` still uses standalone export.

OpenCode's raw JSON events are streamed directly to an ignored file. Each
completed original targeted PNG attachment is immediately saved
as `frame-001.png`, etc., with a SHA-256 and window/action provenance in
`frames.json`. Observation finishes once the image is saved; the native
supervisor then closes this invocation's owned OpenCode/MCP tree. It does not
wait for another model turn to describe the image. `modelRunComplete: false`
records that distinction. Pass `-WaitForReport` when a final model narrative is
needed; the bound still applies. Input runs retain their completed frames even
if a later model response times out; a captured frame is not a completed test.
The supervisor's bounded terminal output is insufficient to
transport large PNG payloads: use the saved files, never reconstruct a truncated
image from terminal text. The adapter fails when no verified targeted PNG exists.
The calling agent can then open the absolute PNG path with its image viewer.

To recover images from an already completed session without recapturing:

```powershell
pwsh -NoProfile -File tools/starsector-proxy/run.ps1 -ExportSession ses_your_completed_session
```

For the qualified finite menu flow, select `-Agent starsector-proxy` and pass
`-Prompt` with the ordered steps, per-step preconditions and expected results,
allowed branches, focus reacquisition permission, action/time bounds and stopping
point. Both `-WaitForReport` and `-BatchAuthorization` are mandatory for new input
runs. The caller's private JSON binds batch B1/B2/B3 to the scenario, plan SHA-256,
current root-issued menu lease and a frozen `driver-B1-owned.private.json` (or B2/B3)
receipt, each with its SHA-256. The owned launch must still be inside its original
480-second bound. Each batch is claimed once as `guard-B1-consumed.private.json`;
failed runs do not regain that authority. The owned MCP child independently
revalidates custody, and rechecks current authority before each attempted action.
Other input scopes need a separately qualified binding; a prompt alone cannot
enable input. Default observation and historical export retain their own scopes.
Do not use `--auto`, resume
an unrelated session or invoke AutoColonyManager's `/ui-smoke`.

The pinned server's guarded embedding rejects overlapping request submissions
and latches later input; its native dispatch FIFO also covers resource observations.
It reserves the closed batch's action attempts before dispatch, never refunds
failed or uncertain attempts, and rejects repetitions, other mutations, wrong
targets and coordinates outside the verified window. B1 permits one optional
activation then Play; B2 one optional activation then two pointer moves; B3 Quit
and an optional expected confirmation. Diagnostic/policy, original PNG receipt,
native width and strict fresh focus checks remain required. A successful activation
acknowledgment is not proof of focus. Guard receipts are private
`batch-dispatch.private.jsonl`; report request overlap, native overlap, rejected
requests and actual action attempts separately. An earlier action may already be
running when a later overlapping request is rejected.

Approval-required input refuses before native effects. This finite flow cannot
continue an interactive approval using older image/focus evidence; it neither
opens a legacy approval interaction nor exposes a resumable modern approval round.
It retains the reserved attempt and input latch. A needed approval requires a
separate qualified continuation with fresh authority and observations.

`guarded-http.mjs` is a transport library for a direct client adapter. It serves
one already-created guarded batch through the pinned SDK's sessionful HTTP
transport on an assigned `127.0.0.1` port. It does not create a native session,
claim authority, grant approval or install a Codex tool. A future owned caller
must retain the existing exclusive batch claim, mutation-time custody checks
and Windows Job deadlines; the library alone is not a native runner.

Each request requires the caller's fresh in-memory 32-byte bearer, the exact
Host and endpoint, and an absent or exact matching Origin. The supported client
uses protocol `2025-11-25` and empty capabilities. Modern envelopes, Tasks,
roots, sampling and elicitation capabilities refuse before dispatch. These
restrictions preserve the existing non-resumable approval behavior; they do not
provide unattended approval. Separate requests retain the same session, guard
state and task manager. The adapter preserves original image content, policy
errors and cancellation. Call the client's `terminateSession()` before closing
its transport; successful DELETE closes the batch and listener. Both `close()`
and `closed` retain shutdown failures. Synthetic HTTP checks do not establish
desktop capture, process ownership or game acceptance.

`guarded-http-child.mjs` adds the owned provider side of that adapter. It requires
an existing claimed batch, source-pinned query helpers, a fresh probe directory
and an inherited bearer, which the CLI removes from its environment before
starting children. A read-only CIM/Job query verifies the child and helper's
PID, birth, executable, Windows session and parent linkage before native session
creation. Readiness contains a bearer digest rather than the bearer. The caller
still must prove its own verifier-to-child linkage and await actual terminal Job
evidence. Readiness after 20 seconds refuses; cooperative probe cancellation
starts then, while general setup and cleanup retain the original 180-second Job
bound. This provider is not a complete direct frontend.

The child defaults to the original approval decline. Its optional
`NF_PROXY_HTTP_CONSENT_MODE=finite-explicit-consent` requires identical
`directHostConsent` objects in a new root-issued plan and lease, each containing
`schemaVersion: 1`, `mode: "finite-explicit-consent"` and
`maxCallsPerBatch` from 1 through 32. The plan, lease and run remain bound to the
existing single-use claim. The environment setting alone grants nothing.
All-call native approval must remain enabled, and a configured approval token
refuses this route.

`finite-consent.mjs` implements the vendor's supported host callback for that
explicit finite consent. Each decision is one-shot and bound to the exact tool,
arguments and cancellation signal in a private active native dispatch, entered
after the existing guard's checks and reservation. It rereads current authority
before every approval; expired, revoked, changed, detached or reused authority
denies. Native blocked-app and emergency-stop policy remains authoritative.
This is a fresh host decision for an already authorized call; no interactive
continuation or cached form approval is used. Existing OpenCode and observation
server behavior stays unchanged. Runtime integration still requires a qualified
frontend, original image custody and separately admitted game scenarios.

The maintained `direct-mcp.mjs` frontend registers 12 vendor tools and the two
`begin_batch`/`finish_batch` controls with the vendor's schemas and metadata.
Advertising a tool grants no input authority: the owned HTTP backend retains
its original 11 raw tools for V1. V2 E1 permits observations and only the exact
`key` call `text: "escape", repeat: 1`; V2 E2 permits observations and its
inspected ordinary Exit `left_click`. Current root-issued authority, original
process identity, strict focus, image custody, deadlines and remaining action
attempts still govern each call. Unknown input or shutdown cannot transfer to E2.

Source qualification and synthetic stdio/HTTP checks do not refresh an already
loaded MCP frontend or establish live desktop capture, game input or exit success.

Fresh V1 menu flows also require the immutable launch-grant composition in
`tools/starsector-proxy/fixture/README.md`. The frozen menu plan and lease bind a
separate launch-only grant; both protected launch gates validate it and the
finite provider requires the actual controller's original lease digest. This
keeps launcher authority and menu input scopes distinct without editing a live
lease. Old independently constructed synthetic launch/input fixtures do not
prove this composition. The change does not qualify V2 campaign handoff.
A supported host or connection refresh and separately admitted native validation
remain required before using newly registered tools in a live scenario.

The guard does not establish visible preconditions, AFTER images, frame freshness
or scenario success. The Codex driver and OpenCode instructions still check those
assertions. A completed runner/export or confident narrative does not qualify a test.

Focused tooling checks (no native product build or game input):

```powershell
node --test tools/starsector-proxy/*.test.mjs
npm run check:types
```

## What an observation proves

The observer has no input, shell, edit, deployment or task tools. It discovers
fresh windows and takes a lossless PNG with `target_window_id` and the returned
window width. Its text report is a secondary interpretation; the calling agent
must inspect the original image before accepting a visible assertion. Keep
private screenshots and audit logs out of Git and public evidence.

Record screenshot call ID, tool arguments, event timestamp, file path and
SHA-256 together. The timestamp identifies the proxy event, not the age of the
game's rendered frame. Identical bytes may mean an unchanged scene, an MCP
cache hit or stale LWJGL capture. Different bytes alone also do not establish
that the requested action succeeded.

The extractor supports retained legacy exports and the installed CLI's typed
assistant content, tool-call IDs and original MCP PNG content. It rejects missing
or conflicting capture IDs, mismatched targets and multiple images. Inline PNG
file wrappers are supported; external file paths and URLs are rejected. A new CLI
still needs an actual image-transport check before native input. The input agent
permits strict targeted `mouse_move` for caller-authorized hover steps; the
observer remains read-only.

## Codex scenario ownership

The orchestrator plans the complete authorized flow before dispatch, then
delegates it to one dedicated Codex game-driver subagent when available and
permitted. Delegation covers proxy calls, original-image inspection, routine
transitions, evidence and recovery within the plan. The orchestrator handles
scope decisions, native-lane allocation and concise milestone/exception reports;
it does not manually supervise each menu click. If delegation is prohibited or
unavailable, document that limitation and follow this workflow directly.

Create an ignored `.tmp/starsector-proxy/scenarios/<scenario-id>/plan.json`
before input and maintain `progress.json` after each completed invocation.
These files are written by the Codex driver; the OpenCode proxy cannot edit files.
Include:

- Objective, current human authorization, disposable fixture and protected saves.
- Ordered step IDs with preconditions, exact action intent, expected visible
  result, permitted branches and the final stopping point.
- Permission to bring the discovered Starsector window forward, allowed focus
  recovery, maximum actions and bounded waits/invocation timeout.
- Save/config backup and autosave-retention checks, plus the authorized exit and
  restoration procedure when relevant.
- Driver identity, native-lane owner and current invocation/session/artifact paths.

Keep the prompt concise. Pass the authorized steps and branches explicitly;
a saved plan, old session or repository document cannot grant action authority.
Use bounded batches ending at meaningful checkpoints within the runner's existing
timeout range. The dedicated driver owns the entire flow across batches; the
orchestrator need not start a new agent or manually approve every routine step.
Keep the model/provider unchanged unless the human explicitly authorizes a
different model/provider. Record that authorization and exact selected model in
the fresh plan. Keep existing process supervision and tool permissions unchanged.

Only one driver may send game input. Coordinate with the orchestrator before
using the shared native verification lane, and release it only after reaping the
owned invocation. Inspect original PNGs after a run completes, never while an
input run is active if that inspection would steal focus. The OpenCode proxy
checks originals between actions inside its batch; the Codex driver independently
reviews every step's images and events before accepting that batch or continuing.

In progress, record each step as `NOT_STARTED`, `INPUT_SENT`, `VERIFIED` or
`UNKNOWN`, with actual parameters, foreground/window identity, assertion,
before/after paths and SHA-256, call IDs, event times and failure reason. Treat
state as evidence about the last run, not proof of the current desktop.
Update a restart checkpoint with the plan/progress/artifact paths, protected-save
status and next eligible step. On restart, rediscover the target and observe
current state before acting. Do not reset or overwrite another driver's records.

Return a compact result: verified steps, unresolved outcome, exact failure
category, evidence paths, process cleanup and save/config restoration status.
Routine PNG inspection belongs to the driver; the orchestrator can inspect any
milestone or exception evidence without duplicating every intermediate review.

## Focus and per-step evidence

Observation does not authorize loading Continue, changing mods, advancing time,
saving, deploying or exiting. Each input must belong to the caller's current
authorized flow. Input-enabled flows explicitly allow focus reacquisition when
the user wants the proxy to bring Starsector forward.

Before any input, query `get_frontmost_app` and rediscover/verify the intended
Starsector window. Before EACH action, recheck foreground PID and exact-window
focus. If Starsector is not foreground, first capture and inspect an original
targeted native-width BEFORE PNG without activating it or capturing the other
app. When authorized, activate only that discovered window once, verify both
foreground identity and focus, then capture and inspect its original AFTER PNG. Do not
assume activation succeeded: `raise_failed` or a mismatch means
`FOCUS_BLOCKED`. Observation-only mode reports the foreground app but never
activates anything. Never capture or send input to an unrelated app.

Once focus is verified, take and inspect an original targeted native-width
BEFORE PNG and require the step's precondition. Recheck focus, window identity and
bounds immediately before sending strict targeted input. If they changed, send
no scenario input; reacquire and recapture within the allowed recovery bound,
or yield. Chat replies, mouse movement and ordinary PC use invalidate previous
focus assumptions. On repeated focus loss, yield without repeatedly stealing
focus or demanding another generic “ready” reply.

After the action, wait the bounded interval, rediscover as needed, check foreground
and focus, and capture/inspect the original targeted AFTER PNG. Require the
expected result before the next listed step or branch. Expected target closure
may be recorded without capturing another app. An uncertain or unexpected
result stops the batch. Never blindly repeat a toggle, save, command or click.

Derive screen-absolute coordinates from current bounds and verified image/client
offsets. Rediscover after launcher handoff, window replacement or invalid handle.
Do not reuse historical AutoColonyManager coordinates. The pinned `zoom` tool
captures the full desktop and remains denied.

Paused Starsector may return old frames. A new capture call, timestamp, FPS label
or changed image hash alone does not establish freshness or success. Report
`CAPTURE_UNCERTAIN` when the visible assertion cannot be verified. Defocus/refocus
requires an explicitly authorized recovery branch; foreground reacquisition
does not authorize interacting with another app. Correlate with fresh game-log
markers or disk evidence when the scenario permits it.

## Timeout and save recovery

Use retained events to distinguish `MODEL_TIMEOUT_BEFORE_INPUT` from
`UNKNOWN_AFTER_INPUT`; a supervisor timeout alone cannot establish which occurred.
Preserve completed frames even if the report/export was interrupted. A timed-out
step with input recorded must be observed before choosing another action; never
repeat it merely because no post-image arrived. If input was not recorded, first
verify there was no pending input call and observe current state before resuming.
Other useful failure categories are `TARGET_AMBIGUOUS`, `PRECONDITION_FAILED`,
`FOCUS_BLOCKED`, `CAPTURE_UNCERTAIN` and `POSTCONDITION_FAILED`. Keep the failure
category separate from the assertion's PASS/FAIL/UNKNOWN result.

A new disposable character does not isolate original autosaves or shared config.
Before launching a scenario, verify original backups and global autosave retention.
Prefer an isolated test installation/save root where supported; otherwise use
the already authorized backup, temporary prevention and restoration procedure.
Do not invent an unsupported save-root switch or edit settings without authority.
Record log baselines and detect rotation before attributing a new event.
Restore protected files only after the actual game has exited and current-file
checks pass, preserving scenario-created data and user display preferences.
See `starsector-debugging.md`.
