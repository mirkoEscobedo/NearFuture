// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdirSync, mkdtempSync, readFileSync, existsSync, rmSync, writeFileSync } from 'node:fs';
import { createHash, randomBytes } from 'node:crypto';
import { resolve, basename, dirname, relative, isAbsolute } from 'node:path';
import { Client, StreamableHTTPClientTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from './batch-authorization.mjs';
import { write, root } from './batch-test-fixture.mjs';
import { startGuardedHttpChild } from './guarded-http-child.mjs';
import { syntheticFixtureBinding } from './fixture-test-binding.mjs';
import { syntheticMenuLaunch } from './menu-launch-test-grant.mjs';
import { menuLaunchSources } from './menu-launch-grant.mjs';

const digest = (/** @type {string} */ path) => createHash('sha256').update(readFileSync(path)).digest('hex');
const sources = menuLaunchSources;
function setup(withConsent = false) {
    // Self-contained filesystem authority with the real admission schema. Probe
    // source bytes are inert: only that external OS boundary is replaced below.
    const syntheticRoot = mkdtempSync(resolve(root, '.tmp/synthetic-http-child-'));
    const scenario = resolve(syntheticRoot, '.tmp/starsector-proxy/scenarios/synthetic');
    mkdirSync(scenario, { recursive: true });
    const paths = { plan: resolve(scenario, 'plan.json'), lease: resolve(scenario, 'native-lane-lease.private.json'),
        owned: resolve(scenario, 'driver-B1-owned.private.json'), fixture: resolve(scenario, 'protected-fixture-receipt.private.json'),
        pins: resolve(scenario, 'frozen-support-pins.private.json'), controller: resolve(scenario, 'controller-start.private.json') };
    for (const path of sources) { const destination = resolve(syntheticRoot, path); mkdirSync(dirname(destination), { recursive: true }); writeFileSync(destination, 'synthetic external probe source'); }
    const consent = { schemaVersion: 1, mode: 'finite-explicit-consent', maxCallsPerBatch: 32 };
    write(paths.plan, { scenarioId: basename(scenario), paths: { scenario }, fixtureBinding: syntheticFixtureBinding(scenario), batches: [{ id: 'B1' }],
        ...(withConsent ? { directHostConsent: consent } : {}), bounds: {
        maxScenarioActionsIncludingActivations: 8, maxProxyInvocations: 3, proxyTimeoutMs: 180000, ownedGameHardTimeoutMs: 480000 } });
    write(paths.fixture, { synthetic: true });
    const now = Date.now();
    const lease = { schemaVersion: 1, authorized: true, phase: 'MENU_SMOKE_ONLY', issuer: '/root', driver: '/root/recovery_game_driver',
        planSha256: digest(paths.plan), ownedGameHardTimeoutMs: 480000, maxActions: 8, maxProxyInvocations: 3,
        campaignLoadSaveAllowed: false, supportPinsPath: paths.pins, supportPinsSha256: '', fixtureReceiptSha256: digest(paths.fixture),
        ...(withConsent ? { directHostConsent: consent } : {}) };
    write(paths.controller, { at: new Date(now - 2000).toISOString(), controllerPid: 100, sessionId: 1, supervisorJobPids: [100, 101],
        hardTimeoutMs: 480000, installation: resolve(scenario, 'installation'), fixtureReceiptSha256: digest(paths.fixture) });
    write(paths.owned, { processes: [{ pid: 101, inSupervisorJob: true, supervisorJobPids: [100, 101], sessionId: 1,
        creationTime: new Date(now - 1000).toISOString(), executable: resolve(scenario, 'installation/java.exe') }] });
    const auth = { schemaVersion: 1, scenario, batch: 'B1', planPath: paths.plan, planSha256: digest(paths.plan),
        leasePath: paths.lease, leaseSha256: '', ownedReceiptPath: paths.owned, ownedReceiptSha256: digest(paths.owned) };
    const f = { scenario, lease, path: resolve(scenario, 'B1-authorization.private.json'),
        refresh() { write(paths.lease, lease); auth.leaseSha256 = digest(paths.lease); write(this.path, auth); } };
    const pins = resolve(f.scenario, 'frozen-support-pins.private.json');
    write(pins, { pins: sources.map(path => ({ path: resolve(syntheticRoot, path), sha256: digest(resolve(syntheticRoot, path)) })) });
    f.lease.supportPinsSha256 = digest(pins); f.refresh();
    Object.assign(f.lease, syntheticMenuLaunch(syntheticRoot, scenario).lease);
    auth.planSha256 = digest(paths.plan); f.refresh();
    const runID = 'http-child-' + randomBytes(8).toString('hex');
    const output = resolve(syntheticRoot, '.tmp/starsector-proxy', runID); mkdirSync(output);
    const probeRoot = mkdtempSync(resolve(syntheticRoot, '.tmp/starsector-proxy/scenarios/synthetic-child-probe-'));
    claimBatchAuthorization(readBatchAuthorization(f.path, { root: syntheticRoot, runID }));
    const options = { root: syntheticRoot, source: f.path, runID, token: randomBytes(32).toString('hex'), probeRoot };
    let probes = 0, sessions = 0;
    const dependencies = {
        probe: async () => { probes++; return { node: { pid: process.pid, parentPid: process.ppid,
            creationTime: new Date(Date.now() - process.uptime() * 1000).toISOString(), executable: process.execPath, sessionId: 1 },
            helper: { pid: 555, parentPid: process.pid, creationTime: new Date().toISOString(), executable: 'synthetic-pwsh', sessionId: 1 },
            supervisorJobPids: [process.pid, 555], query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' }; },
        sessionFactory: () => { sessions++; return /** @type {any} */ ({ dispatch: async (/** @type {string} */ tool) => {
            const target = { pid: 101, windowId: 19, title: 'Starsector Synthetic', bundleId: 'java.exe', isFocused: true,
                bounds: { x: 10, y: 10, width: 100, height: 80 } };
            if (tool === 'screenshot') return { content: [{ type: 'image', mimeType: 'image/png',
                data: 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=' }] };
            const result = tool === 'get_window' ? target : tool === 'get_frontmost_app' ? { app: { pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' } }
                : tool === 'policy_status' ? { profile: 'core' } : { ok: true };
            return { content: [{ type: 'text', text: JSON.stringify(result) }] };
        } }); },
    };
    return { f, options, dependencies, output, probes: () => probes, sessions: () => sessions,
        close: () => { const suffix = relative(resolve(root, '.tmp'), syntheticRoot);
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix)); rmSync(syntheticRoot, { recursive: true }); } };
}

test('owned child preserves real admission, guard, SDK session and nonsecret ready receipt', async () => {
    const f = setup();
    let child;
    const client = new Client({ name: 'child-synthetic', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        child = await startGuardedHttpChild(f.options, f.dependencies);
        const bytes = readFileSync(resolve(f.output, 'guarded-http-ready.private.json'), 'utf8');
        const ready = JSON.parse(bytes);
        assert.equal(bytes.includes(f.options.token), false);
        assert.equal(ready.bearerSha256, createHash('sha256').update(f.options.token).digest('hex'));
        assert.equal(ready.node.pid, process.pid); assert.equal(f.sessions(), 1); assert.equal(f.probes(), 1);
        const transport = new StreamableHTTPClientTransport(new URL(ready.url), {
            requestInit: { headers: { Authorization: 'Bearer ' + f.options.token } },
            reconnectionOptions: { maxRetries: 0, maxReconnectionDelay: 0, initialReconnectionDelay: 0, reconnectionDelayGrowFactor: 1 } });
        await client.connect(transport);
        const response = await client.callTool({ name: 'doctor', arguments: {} });
        assert.notEqual(response.isError, true);
        await transport.terminateSession(); await child.closed;
        assert.equal(f.sessions(), 1);
    } finally { await client.close(); await child?.close(); f.close(); }
});

test('missing claim and changed probe source refuse before probe or native session', async () => {
    for (const change of ['claim', 'pins']) {
        const f = setup();
        try {
            if (change === 'claim') f.options.runID += '-other';
            else {
                const pins = resolve(f.f.scenario, 'frozen-support-pins.private.json');
                write(pins, { pins: [] }); f.f.lease.supportPinsSha256 = digest(pins); f.f.refresh();
            }
            await assert.rejects(startGuardedHttpChild(f.options, f.dependencies));
            assert.equal(f.probes(), 0); assert.equal(f.sessions(), 0);
            assert.equal(existsSync(resolve(f.output, 'batch-dispatch.private.jsonl')), false);
        } finally { f.close(); }
    }
});

for (const change of ['wrong-parent', 'wrong-birth', 'wrong-session', 'wrong-query', 'missing-job-member', 'duplicate-job-member']) {
    test(change + ' rejects the external identity before native creation', async () => {
        const f = setup(), original = f.dependencies.probe;
        f.dependencies.probe = async () => {
            const row = await original();
            if (change === 'wrong-parent') row.helper.parentPid++;
            if (change === 'wrong-birth') row.node.creationTime = new Date(Date.now() + 60000).toISOString();
            if (change === 'wrong-session') row.helper.sessionId++;
            if (change === 'wrong-query') row.query = 'unsupported';
            if (change === 'missing-job-member') row.supervisorJobPids.pop();
            if (change === 'duplicate-job-member') row.supervisorJobPids.push(process.pid);
            return row;
        };
        try {
            await assert.rejects(startGuardedHttpChild(f.options, f.dependencies), /identity required/);
            assert.equal(f.sessions(), 0); assert.equal(existsSync(resolve(f.output, 'guarded-http-ready.private.json')), false);
        } finally { f.close(); }
    });
}

test('lease revoked during ownership probe refuses before native creation', async () => {
    const f = setup(), original = f.dependencies.probe;
    f.dependencies.probe = async () => { const ownership = await original(); f.f.lease.authorized = false; f.f.refresh(); return ownership; };
    try {
        await assert.rejects(startGuardedHttpChild(f.options, f.dependencies));
        assert.equal(f.sessions(), 0); assert.equal(existsSync(resolve(f.output, 'guarded-http-ready.private.json')), false);
    } finally { f.close(); }
});

test('published batch revalidates the original lease at every native mutation', async () => {
    const f = setup();
    let child, mutations = 0;
    const original = f.dependencies.sessionFactory;
    f.dependencies.sessionFactory = () => {
        const session = original(), dispatch = session.dispatch;
        session.dispatch = (/** @type {string} */ tool, /** @type {any[]} */ ...args) => { if (tool === 'left_click') mutations++; return dispatch(tool, ...args); };
        return session;
    };
    const client = new Client({ name: 'mutation-revalidation', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        child = await startGuardedHttpChild(f.options, f.dependencies);
        const ready = JSON.parse(readFileSync(resolve(f.output, 'guarded-http-ready.private.json'), 'utf8'));
        const transport = new StreamableHTTPClientTransport(new URL(ready.url), { requestInit: { headers: { Authorization: 'Bearer ' + f.options.token } },
            reconnectionOptions: { maxRetries: 0, maxReconnectionDelay: 0, initialReconnectionDelay: 0, reconnectionDelayGrowFactor: 1 } });
        await client.connect(transport);
        const call = (/** @type {string} */ name, /** @type {any} */ args = {}) => client.callTool({ name, arguments: args });
        await call('doctor'); await call('policy_status'); await call('get_window', { window_id: 19 });
        await call('screenshot', { target_window_id: 19, quality: 0, width: 100 });
        await call('get_frontmost_app'); await call('get_window', { window_id: 19 });
        f.f.lease.authorized = false; f.f.refresh();
        await assert.rejects(call('left_click', { target_window_id: 19, focus_strategy: 'strict', coordinate: [20, 20] }));
        assert.equal(mutations, 0);
        await transport.terminateSession(); await child.closed;
        const log = readFileSync(resolve(f.output, 'batch-dispatch.private.jsonl'), 'utf8');
        assert.equal(log.includes(f.options.token), false);
        const rows = log.trim().split('\n').map(row => JSON.parse(row));
        assert.equal(rows.filter(row => row.phase === 'before-dispatch' && row.tool === 'left_click').length, 0);
        assert.equal(rows.filter(row => row.phase === 'refused-or-failed' && row.tool === 'left_click').length, 1);
    } finally { await client.close(); await child?.close(); f.close(); }
});

test('factory failure and preexisting readiness preserve evidence and never reuse the claimed log', async () => {
    const f = setup();
    f.dependencies.sessionFactory = () => { throw new Error('Synthetic external native startup failure'); };
    try {
        await assert.rejects(startGuardedHttpChild(f.options, f.dependencies), /Synthetic external/);
        assert.equal(existsSync(resolve(f.output, 'guarded-http-ready.private.json')), false);
        assert.equal(f.probes(), 1);
        await assert.rejects(startGuardedHttpChild(f.options, f.dependencies), /EEXIST/);
        assert.equal(f.probes(), 1);
        writeFileSync(resolve(f.output, 'guarded-http-ready.private.json'), 'retained');
        await assert.rejects(startGuardedHttpChild(f.options, f.dependencies), /Preserve existing/);
        assert.equal(readFileSync(resolve(f.output, 'guarded-http-ready.private.json'), 'utf8'), 'retained');
        assert.equal(f.probes(), 1);
    } finally { f.close(); }
});

test('changed query helper bytes refuse before creating the external probe', async () => {
    const f = setup();
    try {
        writeFileSync(resolve(f.options.root, sources[0]), 'changed probe source');
        await assert.rejects(startGuardedHttpChild(f.options, f.dependencies), /source custody required/);
        assert.equal(f.probes(), 0); assert.equal(f.sessions(), 0);
        assert.equal(existsSync(resolve(f.output, 'batch-dispatch.private.jsonl')), false);
    } finally { f.close(); }
});

test('20-second startup cancellation reaches the pending external probe without creating a native session', { timeout: 25000 }, async () => {
    const f = setup(); let cancelled = false;
    const dependencies = { ...f.dependencies,
        probe: async (/** @type {{signal: AbortSignal}} */ { signal }) => new Promise((done, fail) => {
            signal.addEventListener('abort', () => { cancelled = true; fail(new Error('Synthetic probe cancelled')); }, { once: true });
        }) };
    const started = Date.now();
    try {
        await assert.rejects(startGuardedHttpChild(f.options, dependencies), /Synthetic probe cancelled/);
        assert.equal(cancelled, true); assert.ok(Date.now() - started >= 19900);
        assert.ok(Date.now() - started < 24000); assert.equal(f.sessions(), 0);
        assert.equal(existsSync(resolve(f.output, 'guarded-http-ready.private.json')), false);
    } finally { f.close(); }
});

test('opt-in mode alone grants nothing and refuses missing consent before the OS probe', async () => {
    const f = setup(); const previous = process.env.COMPUTER_USE_REQUIRE_APPROVAL;
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true';
    try {
        await assert.rejects(startGuardedHttpChild({ ...f.options, consentMode: 'finite-explicit-consent' }, f.dependencies), /Explicit current finite host consent/);
        assert.equal(f.probes(), 0); assert.equal(f.sessions(), 0);
        await assert.rejects(startGuardedHttpChild({ ...f.options, consentMode: 'unknown' }, f.dependencies), /Unknown finite consent mode/);
        assert.equal(f.probes(), 0);
    } finally { f.close(); if (previous === undefined) delete process.env.COMPUTER_USE_REQUIRE_APPROVAL; else process.env.COMPUTER_USE_REQUIRE_APPROVAL = previous; }
});

test('explicit consenting child uses the supported one-shot callback and reports its mode', async () => {
    const f = setup(true); let approved = false, child;
    const previous = process.env.COMPUTER_USE_REQUIRE_APPROVAL; process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true';
    const client = new Client({ name: 'consenting-child', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        const dependencies = { ...f.dependencies, sessionFactory: (/** @type {any} */ options) => /** @type {any} */ ({
            dispatch: async (/** @type {string} */ tool, /** @type {any} */ args) => {
                approved = await options.elicitApproval({ tool, args, destructive: false, reasons: ['approval_required_for_all'] });
                return { content: [{ type: 'text', text: JSON.stringify({ ok: approved }) }], ...(!approved ? { isError: true } : {}) };
            },
        }) };
        child = await startGuardedHttpChild({ ...f.options, consentMode: 'finite-explicit-consent' }, dependencies);
        const ready = JSON.parse(readFileSync(resolve(f.output, 'guarded-http-ready.private.json'), 'utf8'));
        assert.equal(ready.consentMode, 'finite-explicit-consent');
        const transport = new StreamableHTTPClientTransport(new URL(ready.url), { requestInit: { headers: { Authorization: 'Bearer ' + f.options.token } },
            reconnectionOptions: { maxRetries: 0, maxReconnectionDelay: 0, initialReconnectionDelay: 0, reconnectionDelayGrowFactor: 1 } });
        await client.connect(transport);
        const result = await client.callTool({ name: 'doctor', arguments: {} });
        assert.equal(approved, true); assert.notEqual(result.isError, true);
        await transport.terminateSession(); await child.closed;
        assert.equal(readFileSync(resolve(f.output, 'batch-dispatch.private.jsonl'), 'utf8').includes(f.options.token), false);
    } finally {
        await client.close(); await child?.close(); f.close();
        if (previous === undefined) delete process.env.COMPUTER_USE_REQUIRE_APPROVAL; else process.env.COMPUTER_USE_REQUIRE_APPROVAL = previous;
    }
});
