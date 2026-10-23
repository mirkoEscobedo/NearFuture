<!-- workspace-template:adaptive-delivery:start -->
## Adaptive Delivery

Use `npm exec -- workspace-template` as this repository's workspace-template entry point. Direct is the default; Ticketed is for multi-session slices; Governed is reserved for enumerated high-consequence authority. Review is read-only, allows at most two semantic repair rounds, and returns `INSUFFICIENT_EVIDENCE` when required inspection is unavailable.

Load package-owned methodology without copying it:

- Behavior changes: `npm exec -- workspace-template skills show tdd` and `npm exec -- workspace-template skills show implementation-style`.
- Diagnosis: `npm exec -- workspace-template skills show diagnose`.
- Spawned commands: `npm exec -- workspace-template skills show process-lifecycle`.
- Completion evidence: `npm exec -- workspace-template skills show verify`.
- Wayfinder only when a route-changing ambiguity remains.
- Ticket compilation only after Ticketed or Governed routing.

Discover the complete inventory with `npm exec -- workspace-template skills list`. Do not copy generic skills into this repository. The host or repository owner selects available models, agents, permissions, skills, and capabilities.
<!-- workspace-template:adaptive-delivery:end -->

## Integration

Follow [the branch/profile policy](docs/delivery/integration.md) for #92.
Ordinary issue branches target current dev; main promotion is separate. Require
independent exact-diff review, current required checks and serial PR landing.
Keep incomplete capabilities disabled and never close a broader parent from a
partial component merge. Use `npm run check:profile` for the accepted headless path.

## Starsector desktop checks

Read `docs/starsector-proxy.md` and `docs/starsector-debugging.md` before live
checks. Use `tools/starsector-proxy/run.ps1` on the signed-in Windows desktop;
the default OpenCode agent observes only.

Before an input scenario, plan the entire authorized flow and delegate it to
one dedicated Codex game-driver subagent when delegation is available and
permitted. The driver owns proxy calls, original-image inspection, routine
transitions and recovery within that plan; the orchestrator receives milestones,
exceptions and final evidence. If delegation is unavailable or prohibited,
record that limitation and follow the same procedure directly; do not create
another user-owned chat as a workaround.

Save the plan and per-step progress under ignored
`.tmp/starsector-proxy/scenarios/<scenario-id>/`, and link them in the restart
checkpoint. Follow the plan/result contract in `docs/starsector-proxy.md`.
The separate OpenCode `starsector-proxy` agent may execute a finite caller-
authorized flow, verifying foreground identity, focus and original before/after
PNGs for every action, then stops at its specified boundary. Reacquire current
state after user interaction; never assume the previous focus or mouse position.
Serialize game drivers and coordinate the native verification lane with the
orchestrator. Inspect saved images rather than trusting text alone, and
distinguish capture receipt, frame freshness and test success. Check global
autosave retention before disposable scenarios; a new character does not isolate
original saves.

Keep private artifacts under ignored `.tmp/starsector-proxy`; do not copy
AutoColonyManager's build/deploy or Continue-save behavior into NearFuture.
