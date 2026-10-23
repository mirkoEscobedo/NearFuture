// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fixture, root, write } from './batch-test-fixture.mjs';
import { claimBatchAuthorization, readBatchAuthorization } from './batch-authorization.mjs';
import { createFiniteConsentMcp } from './finite-consent.mjs';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';

const consent = { schemaVersion: 1, mode: 'finite-explicit-consent', maxCallsPerBatch: 32 };
const target = { pid: 101, windowId: 19, title: 'Starsector', bundleId: 'java.exe', isFocused: true,
    bounds: { x: 10, y: 10, width: 100, height: 80 } };
/** @param {any} value @returns {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
/** @param {any} [planConsent] @param {any} [leaseConsent] */
function authorized(planConsent = consent, leaseConsent = planConsent) {
    const f = fixture(), runID = 'finite-consent-synthetic';
    const planPath = resolve(f.scenario, 'plan.json'), plan = JSON.parse(readFileSync(planPath, 'utf8'));
    plan.directHostConsent = planConsent; write(planPath, plan);
    const hash = createHash('sha256').update(readFileSync(planPath)).digest('hex');
    f.auth.planSha256 = hash; f.lease.planSha256 = hash; Object.assign(f.lease, { directHostConsent: leaseConsent }); f.refresh();
    claimBatchAuthorization(readBatchAuthorization(f.path, { root, runID }));
    return { ...f, runID };
}

/** @param {any} embedded */
async function before(embedded) {
    const context = { mcpReq: { method: 'tools/call', signal: new AbortController().signal,
        requestState: () => undefined, inputResponses: {}, log: async () => {}, notify: async () => {}, envelope: {} } };
    for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
        ['screenshot', { target_window_id: 19, quality: 0, width: 100 }], ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) {
        await embedded.registry.executeRegistered(tool, args, context);
    }
    return context;
}

test('each existing finite call receives a fresh exact decision while guard repetitions still refuse', async () => {
    const f = authorized(); let effects = 0, decisions = 0;
    const previous = process.env.COMPUTER_USE_REQUIRE_APPROVAL;
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true';
    const embedded = createFiniteConsentMcp({ root, source: f.path, runID: f.runID }, {
        sessionFactory: options => ({ dispatch: async (tool, args) => {
            const approved = await options?.elicitApproval?.({ tool, args, destructive: false, reasons: ['approval_required_for_all'] });
            decisions++;
            if (!approved) return { isError: true, content: [{ type: 'text', text: 'approval_required' }] };
            if (tool === 'doctor') return json({ ok: true });
            if (tool === 'policy_status') return json({ profile: 'core' });
            if (tool === 'get_window') return json(target);
            if (tool === 'get_frontmost_app') return json({ app: { pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' } });
            if (tool === 'screenshot') return { content: [{ type: 'image', mimeType: 'image/png', data: 'synthetic-original' }] };
            effects++; return json({ clicked: true });
        } }),
    });
    try {
        const context = await before(embedded);
        const args = { target_window_id: 19, focus_strategy: 'strict', coordinate: [20, 20] };
        await embedded.registry.executeRegistered('left_click', args, context);
        assert.equal(effects, 1); assert.equal(decisions, 7);
        await assert.rejects(embedded.registry.executeRegistered('left_click', args, context), /outside remaining sequence/);
        assert.equal(effects, 1); assert.equal(decisions, 7);
    } finally {
        await embedded.server.close(); f.close();
        if (previous === undefined) delete process.env.COMPUTER_USE_REQUIRE_APPROVAL;
        else process.env.COMPUTER_USE_REQUIRE_APPROVAL = previous;
    }
});

/** @param {() => Promise<void>} action */
async function policy(action) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_BLOCKED_APPS', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    try { await action(); }
    finally { names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; }); }
}

/** Only the native/policy decision boundary is injected.
 * @param {(details: any, approve: any, tool: string, args: any, context?: any) => Promise<boolean>} [decision]
 * @param {any} [native] */
function harness(decision = async (details, approve) => approve(details), native = undefined) {
    const f = authorized(); let effects = 0;
    /** @type {Record<string, unknown>[]} */ const events = [];
    const embedded = createFiniteConsentMcp({ root, source: f.path, runID: f.runID }, {
        record: event => events.push(event), sessionFactory: options => {
            const real = native ? createSession({ ...options, native }) : undefined;
            return { dispatch: async (tool, args, signal, progress, context) => {
                if (real && tool === 'left_click') return real.dispatch(tool, args, signal, progress, context);
                const details = { tool, args, destructive: false, reasons: ['approval_required_for_all'] };
                const approved = await decision(details, context?.elicitApproval ?? options?.elicitApproval, tool, args, context);
                if (!approved) return { isError: true, content: [{ type: 'text', text: 'approval_required' }] };
                if (tool === 'doctor') return json({ ok: true });
                if (tool === 'policy_status') return json({ profile: 'core' });
                if (tool === 'get_window') return json(target);
                if (tool === 'get_frontmost_app') return json({ app: { pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' } });
                if (tool === 'screenshot') return { content: [{ type: 'image', mimeType: 'image/png', data: 'synthetic-original' }] };
                if (tool === 'left_click') effects++;
                return json({ ok: true });
            } };
        },
    });
    return { f, embedded, events, effects: () => effects,
        close: async () => { await embedded.server.close(); f.close(); } };
}

test('missing, mismatched or unbounded grants deny before creating a native session', async () => policy(async () => {
    for (const [plan, lease] of [[null, null], [consent, { ...consent, maxCallsPerBatch: 1 }],
        [{ ...consent, maxCallsPerBatch: 33 }, undefined], [{ ...consent, unexpected: true }, undefined]]) {
        const f = authorized(plan, lease); let factories = 0;
        try {
            assert.throws(() => createFiniteConsentMcp({ root, source: f.path, runID: f.runID }, {
                sessionFactory: () => { factories++; throw new Error('Must not create native'); },
            }), /Explicit current finite host consent/);
            assert.equal(factories, 0);
        } finally { f.close(); }
    }
}));

test('lowered approval policy or configured token cannot enable this route', async () => policy(async () => {
    const f = authorized();
    try {
        process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'false';
        assert.throws(() => createFiniteConsentMcp({ root, source: f.path, runID: f.runID }), /unchanged all-call approval/);
        process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; process.env.COMPUTER_USE_APPROVAL_TOKEN = 'synthetic-test-token';
        assert.throws(() => createFiniteConsentMcp({ root, source: f.path, runID: f.runID }), /token shortcut/);
    } finally { f.close(); }
}));

for (const changed of ['revoked', 'expired', 'plan-drift']) {
    test('current authority is revalidated before the exact ' + changed + ' action', async () => policy(async () => {
        const h = harness();
        try {
            const context = await before(h.embedded);
            if (changed === 'revoked') { h.f.lease.authorized = false; write(h.f.leasePath, h.f.lease); }
            else if (changed === 'expired') { h.f.controller.at = new Date(Date.now() - 480001).toISOString(); write(h.f.controllerPath, h.f.controller); }
            else { write(resolve(h.f.scenario, 'plan.json'), { changed: true }); }
            const result = await h.embedded.registry.executeRegistered('left_click', {
                target_window_id: 19, focus_strategy: 'strict', coordinate: [20, 20],
            }, context);
            assert.equal(result.isError, true); assert.equal(h.effects(), 0);
            const decision = h.events.filter(row => row.phase === 'host-consent-decision').at(-1);
            assert.equal(decision?.tool, 'left_click'); assert.equal(decision?.approved, false);
        } finally { await h.close(); }
    }));
}

for (const details of [{ tool: 'run_script' }, { destructive: true }, { targetApp: 'CredentialUIBroker.exe' },
    { reasons: ['sensitive_app:java.exe'] }]) {
    test('a native decision outside finite consent remains denied: ' + JSON.stringify(details), async () => policy(async () => {
        const h = harness(async (current, approve) => approve({ ...current, ...details }));
        try {
            const result = await h.embedded.registry.executeRegistered('doctor', {}, {
                mcpReq: { signal: new AbortController().signal, envelope: {}, requestState: () => undefined },
            });
            assert.equal(result.isError, true); assert.equal(h.effects(), 0);
            assert.ok(h.events.some(row => row.phase === 'host-consent-decision' && row.approved === false));
        } finally { await h.close(); }
    }));
}

test('an inherited asynchronous context cannot approve after its native dispatch has ended', async () => policy(async () => {
    /** @type {Promise<boolean> | undefined} */ let later;
    const h = harness(async (details, approve) => {
        later = new Promise(done => setImmediate(async () => done(await approve(details))));
        return false; // No callback was used during this synthetic dispatch.
    });
    try {
        await h.embedded.registry.executeRegistered('doctor', {}, {
            mcpReq: { signal: new AbortController().signal, envelope: {}, requestState: () => undefined },
        });
        assert.equal(await later, false); assert.equal(h.effects(), 0);
        assert.equal(h.events.filter(row => row.phase === 'host-consent-decision').length, 0);
    } finally { await h.close(); }
}));

test('cancellation at the guarded boundary cannot obtain host consent', async () => policy(async () => {
    const h = harness(), controller = new AbortController(); controller.abort();
    try {
        await assert.rejects(h.embedded.registry.executeRegistered('doctor', {}, {
            mcpReq: { signal: controller.signal, envelope: {}, requestState: () => undefined },
        }), /cancelled/);
        assert.equal(h.events.filter(row => row.phase === 'host-consent-decision').length, 0);
        assert.equal(h.effects(), 0);
    } finally { await h.close(); }
}));

test('mismatched arguments consume the decision and neither retry nor detached callback can reuse it', async () => policy(async () => {
    /** @type {any} */ let saved;
    /** @type {boolean[]} */ const decisions = [];
    const h = harness(async (details, approve) => {
        saved = approve;
        decisions.push(await approve({ ...details, args: { unexpected: true } }));
        decisions.push(await approve(details));
        return false;
    });
    try {
        const context = { mcpReq: { signal: new AbortController().signal, envelope: {}, requestState: () => undefined } };
        const result = await h.embedded.registry.executeRegistered('doctor', {}, context);
        assert.equal(result.isError, true); assert.deepEqual(decisions, [false, false]);
        assert.equal(await saved({ tool: 'doctor', args: {}, reasons: ['approval_required_for_all'], destructive: false }), false);
        assert.equal(h.effects(), 0);
    } finally { await h.close(); }
}));

test('a successful decision is one-shot and cannot approve a second effect', async () => policy(async () => {
    /** @type {boolean[][]} */ const decisions = [];
    const h = harness(async (details, approve) => {
        const one = await approve(details), two = await approve(details);
        decisions.push([one, two]); return one;
    });
    try {
        await before(h.embedded);
        assert.deepEqual(decisions, Array.from({ length: 6 }, () => [true, false]));
        assert.equal(h.effects(), 0);
    } finally { await h.close(); }
}));

test('the per-batch decision ceiling cannot reset between calls', async () => policy(async () => {
    const h = harness();
    const context = { mcpReq: { signal: new AbortController().signal, envelope: {}, requestState: () => undefined } };
    try {
        for (let i = 0; i < 32; i++) assert.notEqual((await h.embedded.registry.executeRegistered('doctor', {}, context)).isError, true);
        assert.equal((await h.embedded.registry.executeRegistered('doctor', {}, context)).isError, true);
        assert.equal(h.events.filter(row => row.phase === 'host-consent-decision' && row.approved === true).length, 32);
    } finally { await h.close(); }
}));

for (const denial of ['blocked-app', 'emergency-stop']) {
    test('real vendor ' + denial + ' remains authoritative before host approval', async () => policy(async () => {
        if (denial === 'blocked-app') process.env.COMPUTER_USE_BLOCKED_APPS = 'java.exe';
        const native = { getWindow: () => target, isNativeEmergencyStopActive: () => denial === 'emergency-stop' };
        const h = harness(undefined, native);
        try {
            const context = await before(h.embedded);
            const decisions = h.events.filter(row => row.phase === 'host-consent-decision').length;
            const result = await h.embedded.registry.executeRegistered('left_click', {
                target_window_id: 19, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [20, 20],
            }, context);
            assert.equal(result.isError, true); assert.equal(h.effects(), 0);
            assert.equal(h.events.filter(row => row.phase === 'host-consent-decision').length, decisions);
            assert.match(JSON.stringify(result), denial === 'blocked-app' ? /target_app_blocked/ : /emergency_stop_active/);
        } finally { await h.close(); }
    }));
}

test('real vendor policy approves one freshly guarded synthetic click and preserves its native result', async () => policy(async () => {
    /** @type {unknown[][]} */ const moves = [];
    /** @type {unknown[][]} */ const clicks = [];
    const native = { getWindow: () => ({ ...target, isOnScreen: true }),
        getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' }),
        getDisplaySize: () => ({ width: 200, height: 200 }), isNativeEmergencyStopActive: () => false,
        mouseMove: (/** @type {number} */ x, /** @type {number} */ y) => { moves.push([x, y]); },
        mouseClick: (/** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ button, /** @type {number} */ count) => { clicks.push([x, y, button, count]); } };
    const h = harness(undefined, native);
    try {
        const context = await before(h.embedded);
        const args = { target_window_id: 19, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [20, 20] };
        const result = await h.embedded.registry.executeRegistered('left_click', args, context);
        assert.notEqual(result.isError, true); assert.match(JSON.stringify(result), /Clicked \(20, 20\)/);
        assert.deepEqual(moves, [[20, 20]]); assert.deepEqual(clicks, [[20, 20, 'left', 1]]);
        assert.ok(h.events.some(row => row.phase === 'host-consent-decision' && row.tool === 'left_click' && row.approved === true));
        await assert.rejects(h.embedded.registry.executeRegistered('left_click', args, context), /outside remaining sequence/);
        assert.equal(clicks.length, 1);
    } finally { await h.close(); }
}));
