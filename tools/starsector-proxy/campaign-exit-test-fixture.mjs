// @ts-check
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync, rmSync, realpathSync } from 'node:fs';
import { resolve, basename, relative, isAbsolute } from 'node:path';
import { campaignFixture, target, escapeArgs, sha } from './campaign-test-fixture.mjs';
import { write } from './batch-test-fixture.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from './batch-authorization.mjs';
import { createFiniteConsentMcp } from './finite-consent.mjs';
import { persistDirectFrame } from './direct-frames.mjs';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';

const original = { content: [{ type: 'image', mimeType: 'image/png',
    data: readFileSync(new URL('./capture/fixtures/readiness.synthetic.png', import.meta.url)).toString('base64') }] };
/** @param {any} value @returns {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
export const exitArgs = { target_window_id: 19, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] };

/** Real guard/finite consent/vendor input path. Only native effects, external
 * frame/metadata observations and provider-closure observations are synthetic. */
export function exitFixture() {
    const f = campaignFixture();
    const nodeExecutable = realpathSync.native(process.execPath);
    // Process/native effects are replaced; these identities are inert fixture inputs.
    const powershell = resolve(f.scenario, 'synthetic-pwsh.exe');
    const supervisor = resolve(f.scenario, 'synthetic-supervisor.exe');
    writeFileSync(powershell, 'synthetic PowerShell identity, never executed');
    writeFileSync(supervisor, 'synthetic supervisor identity, never executed');
    f.runID = 'synthetic-campaign-exit-' + basename(f.scenario);
    const outputParent = resolve(f.root, '.tmp/starsector-proxy'), providerOutput = resolve(outputParent, f.runID);
    const pinsPath = resolve(f.scenario, 'frozen-support-pins.private.json');
    write(pinsPath, { pins: [nodeExecutable, powershell].map(path => ({ path: resolve(path), sha256: sha(path) })) });
    f.lease.supportPinsSha256 = sha(pinsPath); write(f.leasePath, f.lease);
    f.auth.leaseSha256 = sha(f.leasePath); write(f.source, f.auth);
    /** @type {unknown[][]} */ const keys = [], moves = /** @type {number[][]} */ ([]),
        clicks = /** @type {[number, number, string, number][]} */ ([]);
    /** @type {ReturnType<typeof createFiniteConsentMcp> | undefined} */ let embedded;
    let outputCreated = false;
    const context = { mcpReq: { method: 'tools/call', signal: new AbortController().signal,
        requestState: () => undefined, inputResponses: {}, log: async () => {}, notify: async () => {}, envelope: {} } };
    /** @param {string} source @param {string} runID @param {() => void} [beforeClick] */
    function open(source, runID, beforeClick = () => {}) {
        const native = { getWindow: () => ({ ...target, displayName: 'Synthetic', displayId: 1 }),
            getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' }),
            getDisplaySize: () => ({ width: 640, height: 480 }), isNativeEmergencyStopActive: () => false,
            keyPress: (/** @type {string} */ text, /** @type {number | undefined} */ repeat) => { keys.push([text, repeat]); },
            mouseMove: (/** @type {number} */ x, /** @type {number} */ y) => { moves.push([x, y]); },
            mouseClick: (/** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ button, /** @type {number} */ count) => {
                beforeClick(); clicks.push([x, y, button, count]);
            } };
        embedded = createFiniteConsentMcp({ root: f.root, source, runID }, { sessionFactory: options => {
            const real = createSession(/** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').SessionOptions} */
                (/** @type {unknown} */ ({ ...options, native })));
            return { ...real, dispatch: async (tool, args, signal, progress, request) => {
                if (tool === 'key' || tool === 'left_click') return real.dispatch(tool, args, signal, progress, request);
                const approved = await options?.elicitApproval?.({ tool, args, destructive: false,
                    reasons: ['approval_required_for_all'] });
                assert.equal(approved, true, 'Current finite observation consent is required');
                if (tool === 'doctor') return json({ ok: true });
                if (tool === 'policy_status') return json({ profile: 'core' });
                if (tool === 'get_window') return json(target);
                if (tool === 'get_frontmost_app') return json({ app: native.getFrontmostApp() });
                if (tool === 'screenshot') return /** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */ (original);
                throw new Error('Unexpected external observation');
            } };
        } });
        return embedded;
    }
    /** @param {string} tool @param {any} args */
    const call = (tool, args) => {
        assert.ok(embedded, 'An owned synthetic provider must be open');
        return embedded.registry.executeRegistered(tool, args, context);
    };
    async function before() {
        for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
            ['screenshot', { target_window_id: 19, quality: 0, width: 480 }],
            ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) await call(/** @type {string} */ (tool), args);
    }
    async function closeProvider() { await embedded?.server.close(); embedded = undefined; }
    /** Existing E1 behavior is setup; no new E2 effect is hidden in this helper. */
    async function completeEscape() {
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        open(f.source, f.runID); await before();
        assert.notEqual((await call('key', escapeArgs)).isError, true, 'Qualified E1 prerequisite must succeed');
        await call('get_window', { window_id: 19 });
        const args = { target_window_id: 19, quality: 0, width: 480 }, result = await call('screenshot', args);
        const capturedAtMs = Date.now(), state = f.history(), e1 = state.pending;
        const frameOutput = resolve(f.scenario, 'E1-after'); mkdirSync(frameOutput);
        const frame = persistDirectFrame({ output: frameOutput, label: 'capture-001', pngName: 'F1.original.png',
            args, result, window: target, ownedPids: [101], provenance: { runID: f.runID, batch: 'E1',
                sourceSha256: sha(f.source), reservationID: e1.reservationID, capturedAtMs } });
        assert.ok(frame.pngPath, 'Complete original frame prerequisite');
        await closeProvider(); // No concurrent synthetic providers. Closure below is an external process boundary.
        mkdirSync(providerOutput); outputCreated = true;
        const stamp = new Date().toISOString(), creationTime = new Date(state.providerClock.startMs).toISOString();
        const child = { command: nodeExecutable, args: [resolve(f.root, 'tools/starsector-proxy/guarded-http-child.mjs')],
            cwd: '.', processOwnership: 'windows-job-object', status: 0, timedOut: false };
        const ready = { schemaVersion: 1, at: creationTime, runID: f.runID, batch: 'E1', sourceSha256: sha(f.source),
            bearerSha256: 'a'.repeat(64), startupTimeoutMs: 20000, consentMode: 'finite-explicit-consent', url: 'http://127.0.0.1:49153/mcp',
            node: { pid: 39002, parentPid: 39001, creationTime, executable: nodeExecutable, sessionId: 1 },
            helper: { pid: 39003, parentPid: 39002, creationTime, executable: powershell, sessionId: 1 }, supervisorJobPids: [39002, 39003],
            query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' };
        const terminalPath = resolve(providerOutput, 'owned-provider-terminal.private.json');
        const terminal = { at: stamp, verifierPid: 39001, verifierStatus: 0, signal: null,
            deadline: new Date(state.providerClock.deadlineMs).toISOString(), closeFailed: false, overflow: false, spawnFailed: false,
            terminalObserved: true, cleanupUnknown: false, steps: [child], terminalBoundToChild: true, verdict: 'PASS', terminalReceipt: terminalPath };
        const readyPath = resolve(providerOutput, 'guarded-http-ready.private.json'); write(readyPath, ready); write(terminalPath, terminal);
        write(resolve(providerOutput, 'owned-provider.stdout.private.txt'), { schemaVersion: 1, ok: true, command: 'verify', result: { verdict: 'PASS', steps: [child] } });
        // Preserve the provider's ordinary empty stderr contract without fabricating a real process.
        writeFileSync(resolve(providerOutput, 'owned-provider.stderr.private.txt'), '');
        const receipt = { schemaVersion: 1, kind: 'STARSECTOR_E1_MENU_INSPECTION', scenario: f.scenario,
            flow: f.campaign.flow, rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
            e1: { sourceSha256: sha(f.source), claimSha256: sha(resolve(f.scenario, 'guard-E1-consumed.private.json')),
                runID: f.runID, reservationID: e1.reservationID },
            frame: { pngPath: frame.pngPath, pngSha256: sha(frame.pngPath), metadataPath: frame.metadataPath,
                metadataSha256: sha(frame.metadataPath), capturedAtMs },
            inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(), state: 'ORDINARY_GAME_MENU',
                exitCoordinate: [...exitArgs.coordinate] },
            provider: { readySha256: sha(readyPath), terminalSha256: sha(terminalPath), observedClosedAtMs: Date.now(), providerReaped: true } };
        return { receipt, state, terminal, terminalPath, ready, readyPath, originalE1SourceSha256: sha(f.source) };
    }
    /** Root data staging only; never claims or sends E2 input.
     * @param {Pick<Awaited<ReturnType<typeof completeEscape>>, 'receipt'>} completed */
    function stageExit(completed) {
        const menuPath = resolve(f.scenario, 'E1-menu-inspected.private.json'); write(menuPath, completed.receipt);
        const plan = structuredClone(f.plan); plan.e1MenuReceipt = { path: menuPath, sha256: sha(menuPath) };
        const planPath = resolve(f.scenario, 'plan.json'); write(planPath, plan);
        const lease = { ...f.lease, campaign: plan.campaign, planSha256: sha(planPath), e1MenuReceipt: plan.e1MenuReceipt };
        write(f.leasePath, lease);
        const ownedReceiptPath = resolve(f.scenario, 'driver-E2-owned.private.json');
        write(ownedReceiptPath, JSON.parse(readFileSync(f.auth.ownedReceiptPath, 'utf8')));
        const source = resolve(f.scenario, 'E2-authorization.private.json');
        write(source, { ...f.auth, batch: 'E2', planSha256: sha(planPath), leaseSha256: sha(f.leasePath),
            ownedReceiptPath, ownedReceiptSha256: sha(ownedReceiptPath) });
        return { source, runID: f.runID + '-exit', plan, lease, menuPath };
    }
    return { ...f, powershell, supervisor, providerOutput, keys, moves, clicks, open, call, before, completeEscape, stageExit, closeProvider,
        async close() {
            await closeProvider();
            if (outputCreated) {
                const suffix = relative(outputParent, providerOutput);
                assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix) && basename(providerOutput).startsWith('synthetic-campaign-exit-'));
                rmSync(providerOutput, { recursive: true, force: true });
            }
            f.close();
        } };
}
