// @ts-check
// Actual stdio/HTTP, claims, guard, consent, vendor and capture artifacts. Only
// OS/native effects and the trusted driver's visual interpretation are generated.
import assert from 'node:assert/strict';
import test from 'node:test';
import { PassThrough } from 'node:stream';
import { mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } from 'node:fs';
import { resolve, basename, relative, isAbsolute } from 'node:path';
import { Client } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { serveDirectMcp } from '../direct-mcp.mjs';
import { directFixture } from '../direct-test-fixture.mjs';
import { requiredOwnedHttpFiles } from '../owned-http-batch.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedNativeSessionFactory, seedTarget } from './campaign-input-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
/** @param {string} path */
const sha = path => filePin(path).sha256.toLowerCase();
/** @param {any} result */
const textJson = result => JSON.parse(result.content.find((/** @type {any} */ item) => item.type === 'text').text);

test('actual finite stdio and HTTP compose first-page Play and one inspected New Game click with the original clipped clock', async () => {
    const f = campaignLaunchFixture({ inspection: true });
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof directFixture> | undefined} */ let provider;
    /** @type {ReturnType<typeof serveDirectMcp> | undefined} */ let frontend;
    const input = new PassThrough(), output = new PassThrough();
    const client = new Client({ name: 'seeded-stdio-composition', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const prefix = 'synthetic-first-page-transport-' + basename(f.scenario);
    const parent = resolve(f.root, '.tmp/starsector-proxy');
    /** @type {string[]} */ const outputs = [];
    /** @type {string[]} */ const effects = [];
    let window = { ...seedTarget }, batch = 'B1', runID = prefix + '-B1', source = '', calls = 0;
    const ledger = () => JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
    /** @param {string} name @param {any} args */
    async function call(name, args) {
        if (!['begin_batch', 'finish_batch'].includes(name)) calls++;
        const result = await client.callTool({ name, arguments: args });
        assert.notEqual(result.isError, true, name + ': ' + JSON.stringify(result)); return result;
    }
    /** @param {boolean} [diagnose] */
    async function before(diagnose = false) {
        if (diagnose) { await call('doctor', {}); await call('policy_status', {}); }
        await call('get_window', { window_id: window.windowId });
        await call('screenshot', { target_window_id: window.windowId, quality: 0, width: 480 });
        await call('get_frontmost_app', {}); await call('get_window', { window_id: window.windowId });
    }
    /** @param {string} directory */
    async function fixedAfter(directory) {
        window = textJson(await call('get_window', { window_id: window.windowId }));
        const args = { target_window_id: window.windowId, quality: 0, width: 480 };
        const result = await call('screenshot', args), label = 'call-' + String(calls).padStart(3, '0');
        const providerOutput = resolve(parent, runID);
        const post = JSON.parse(readFileSync(resolve(providerOutput, label + '.private.json'), 'utf8'));
        const attempt = JSON.parse(readFileSync(resolve(providerOutput, label + '.attempt.private.json'), 'utf8'));
        assert.equal(typeof post.outerProtocolID, 'number'); assert.equal(typeof post.innerProtocolID, 'number');
        assert.equal(post.tool, 'screenshot'); assert.equal(attempt.sourceSha256, sha(source));
        const action = ledger().pending, capturedAtMs = Date.parse(post.callCompleted);
        assert.equal(action.sourceSha256, attempt.sourceSha256); assert.ok(capturedAtMs >= action.knownResultAtMs);
        const frameOutput = resolve(f.scenario, directory); mkdirSync(frameOutput);
        const frame = persistDirectFrame({ output: frameOutput, label: 'capture-001', pngName: 'F1.original.png',
            result, args, window, ownedPids: [101], provenance: { ...post, sourceSha256: attempt.sourceSha256,
                reservationID: action.reservationID, capturedAtMs } }); assert.ok(frame.pngPath);
        assert.deepEqual(readFileSync(frame.rawPath), readFileSync(resolve(providerOutput, label + '.raw.private.json')));
        return { pngPath: frame.pngPath, pngSha256: sha(frame.pngPath), metadataPath: frame.metadataPath,
            metadataSha256: sha(frame.metadataPath), capturedAtMs };
    }
    /** @param {string} phase @param {any} extra */
    function stagePhase(phase, extra) {
        const campaign = { ...f.campaign, flow: 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE', windowId: window.windowId };
        const planPath = resolve(f.scenario, phase + '-plan.private.json');
        const leasePath = resolve(f.scenario, phase + '-lease.private.json');
        json(planPath, { ...f.phasePlan, campaign, batches: [{ id: phase }], ...extra });
        json(leasePath, { ...f.phaseLease, schemaVersion: 4, phase: campaign.flow, campaign,
            planSha256: sha(planPath), ...extra });
        const ownedReceiptPath = resolve(f.scenario, 'driver-' + phase + '-owned.private.json');
        if (phase !== 'E1') json(ownedReceiptPath,
            JSON.parse(readFileSync(resolve(f.scenario, 'driver-E1-owned.private.json'), 'utf8')));
        const path = resolve(f.scenario, phase + '-authorization.private.json');
        json(path, { schemaVersion: 4, scenario: f.scenario, batch: phase, planPath, planSha256: sha(planPath),
            leasePath, leaseSha256: sha(leasePath), ownedReceiptPath, ownedReceiptSha256: sha(ownedReceiptPath) }); return path;
    }
    /** @param {string} phase @param {string} path */
    async function begin(phase, path) {
        batch = phase; source = path; runID = prefix + '-' + phase; calls = 0; outputs.push(resolve(parent, runID));
        const started = textJson(await call('begin_batch', { source, runID, consentMode: 'finite-explicit-consent' }));
        assert.equal(Date.parse(started.deadline), f.campaign.rootClock.callerAdmissionDeadlineMs - 30000);
    }
    async function finish() {
        const terminal = textJson(await call('finish_batch', {}));
        assert.equal(terminal.terminalObserved, true); assert.equal(terminal.cleanupUnknown, false);
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.terminalBoundToChild, true);
        const path = resolve(parent, runID);
        return { readySha256: sha(resolve(path, 'guarded-http-ready.private.json')),
            terminalSha256: sha(resolve(path, 'owned-provider-terminal.private.json')),
            observedClosedAtMs: Date.now(), providerReaped: true };
    }
    try {
        const powershell = resolve(f.scenario, 'synthetic-transport-helper.exe');
        const supervisor = resolve(f.scenario, 'synthetic-transport-verifier.exe');
        for (const path of [powershell, supervisor]) writeFileSync(path, 'Generated OS identity. Never executed.');
        const pinsPath = resolve(f.scenario, 'frozen-support-pins.private.json');
        json(pinsPath, { pins: [...new Set(requiredOwnedHttpFiles(f.root, powershell, supervisor))].map(path => ({ path, sha256: sha(path) })) });
        f.phaseLease.supportPinsSha256 = f.launchLease.supportPinsSha256 = sha(pinsPath);
        const callerCap = Date.now() + 170000;
        f.plan.callerAdmissionDeadlineMs = f.launchLease.callerAdmissionDeadlineMs = callerCap;
        Object.assign(f.campaign.rootClock, { callerAdmissionDeadlineMs: callerCap,
            originalCallerAdmissionDeadlineMs: callerCap, effectiveDeadlineMs: callerCap });
        f.campaign.producerWitness.effectiveDeadlineMs = callerCap;
        f.refreshLaunch();
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const b1 = claimSeededLauncher(f, runID, false);
        provider = directFixture({ authority: { root: f.root, source: b1.source, runID, powershell, supervisor, consentMode: 'finite-explicit-consent' },
            sessionFactory: seedNativeSessionFactory(tool => {
                assert.equal(ledger().history.sentActions, effects.length + 1, 'guard marks exact consent before every native effect');
                effects.push(tool);
            }, { window: () => window }) });
        frontend = serveDirectMcp({ ...provider.options, spawn: provider.spawn, stdin: input, stdout: output });
        await client.connect(new StdioServerTransport(output, input));
        await begin('B1', b1.source); await before(true);
        await call('left_click', { target_window_id: 19, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] });
        window = { ...seedTarget, windowId: 21 };
        const b1Frame = await fixedAfter('B1-after'), b1Action = ledger().pending;
        const b1Provider = await finish();
        const b1ReceiptPath = resolve(f.scenario, 'B1-menu-inspected.private.json');
        json(b1ReceiptPath, { schemaVersion: 1, kind: 'STARSECTOR_B1_MAIN_MENU_INSPECTION', scenario: f.scenario,
            flow: 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE', rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
            b1: { sourceSha256: sha(source), claimSha256: sha(resolve(f.scenario, 'guard-B1-consumed.private.json')),
                runID, reservationID: b1Action.reservationID }, frame: b1Frame,
            inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                state: 'STARSECTOR_MAIN_MENU', newGameCoordinate: [40, 80] }, provider: b1Provider });
        await begin('E1', stagePhase('E1', { b1MenuReceipt: { path: b1ReceiptPath, sha256: sha(b1ReceiptPath) } }));
        await before(true);
        await call('left_click', { target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] });
        const firstPageFrame = await fixedAfter('E1-first-page');
        assert.ok(firstPageFrame.pngPath);
        await finish();
        assert.deepEqual(effects, ['left_click', 'left_click']);
        assert.equal(provider.starts(), 2);
        assert.equal(ledger().history.reservedActions, 2);
        assert.equal(ledger().history.sentActions, 2);
        assert.equal(ledger().uncertain, false);
        assert.equal(ledger().pending.batch, 'E1');
        assert.equal(ledger().pending.status, 'AWAITING_AFTER');
        assert.equal(ledger().resolvedB1.batch, 'B1');
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
    } finally {
        await client.close(); await frontend?.close(); await provider?.close();
        for (const path of outputs) if (existsSync(path)) {
            const suffix = relative(parent, path); assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix));
            rmSync(path, { recursive: true });
        }
        f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
