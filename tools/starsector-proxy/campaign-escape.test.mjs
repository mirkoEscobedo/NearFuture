// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { campaignFixture, target, escapeArgs, sha } from './campaign-test-fixture.mjs';
import { write } from './batch-test-fixture.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';
import { createBatchSession } from './batch-session.mjs';
import { createFiniteConsentMcp } from './finite-consent.mjs';
import { createSession } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js';

/** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */
const original = { content: [{ type: 'image', mimeType: 'image/png',
    data: readFileSync(new URL('./capture/fixtures/readiness.synthetic.png', import.meta.url)).toString('base64') }] };
/** @param {any} value @returns {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').ToolResult} */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
/** @param {any} embedded @param {any} context */
async function freshBefore(embedded, context) {
    for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
        ['screenshot', { target_window_id: 19, quality: 0, width: 480 }],
        ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) {
        await embedded.registry.executeRegistered(tool, args, context);
    }
}

/** Same real finite policy and vendor input path as the positive case; only
 * external native execution and original frame/metadata observations are supplied.
 * @param {ReturnType<typeof campaignFixture>} f
 * @param {(text: string, repeat?: number) => void} keyPress */
function openCampaign(f, keyPress) {
    /** @type {Record<string, unknown>[]} */ const decisions = [];
    /** @type {Pick<import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/native.js').NativeModule,
     * 'getWindow' | 'getFrontmostApp' | 'isNativeEmergencyStopActive' | 'keyPress'>} */
    const native = { getWindow: () => /** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/native.js').WindowRecord} */
        (/** @type {unknown} */ (target)),
        getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' }),
        isNativeEmergencyStopActive: () => false, keyPress };
    const embedded = createFiniteConsentMcp({ root: f.root, source: f.source, runID: f.runID }, {
        record: event => decisions.push(event), sessionFactory: options => {
            // Only these four native methods are reachable in this synthetic adapter.
            const real = createSession(/** @type {import('../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session.js').SessionOptions} */
                (/** @type {unknown} */ ({ ...options, native })));
            return { ...real, dispatch: async (tool, args, signal, progress, context) => {
                if (tool === 'key') return real.dispatch(tool, args, signal, progress, context);
                const approved = await options?.elicitApproval?.({ tool, args, destructive: false,
                    reasons: ['approval_required_for_all'] });
                assert.equal(approved, true);
                if (tool === 'doctor') return json({ ok: true });
                if (tool === 'policy_status') return json({ profile: 'core' });
                if (tool === 'get_window') return json(target);
                if (tool === 'get_frontmost_app') return json({ app: native.getFrontmostApp() });
                if (tool === 'screenshot') return original;
                throw new Error('Unexpected synthetic observation');
            } };
        },
    });
    const context = { mcpReq: { method: 'tools/call', signal: new AbortController().signal,
        requestState: () => undefined, inputResponses: {}, log: async () => {}, notify: async () => {}, envelope: {} } };
    return { embedded, decisions, context };
}

test('V2 reserves inherited launch history before one exact vendor Escape and permits its own AFTER proof', async () => {
    const f = campaignFixture();
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {unknown[][]} */ const effects = [];
    /** @type {ReturnType<typeof createFiniteConsentMcp> | undefined} */ let embedded;
    try {
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        const h = openCampaign(f, (text, repeat) => {
            const state = f.history();
            assert.equal(state.history.reservedActions, 57);
            assert.equal(state.history.sentActions, 55);
            assert.equal(state.uncertain, false);
            assert.equal(state.pending.batch, 'E1');
            assert.equal(state.pending.runID, f.runID);
            assert.equal(state.pending.sourceSha256, sha(f.source));
            // The already reserved owner must survive child/policy custody rereads.
            readClaimedBatchAuthorization(f.source, { root: f.root, runID: f.runID });
            effects.push([text, repeat]);
        });
        embedded = h.embedded;
        const { context, decisions } = h;
        await freshBefore(embedded, context);
        const result = await embedded.registry.executeRegistered('key', escapeArgs, context);
        assert.notEqual(result.isError, true); assert.match(JSON.stringify(result), /Pressed escape/);
        assert.deepEqual(effects, [['escape', 1]]);
        assert.ok(decisions.some(row => row.phase === 'host-consent-decision' && row.tool === 'key' && row.approved === true));
        assert.equal(f.history().pending.status, 'AWAITING_AFTER');
        assert.equal(f.history().uncertain, false);
        // Post-input evidence belongs to the same owner; pending input cannot deny it.
        await embedded.registry.executeRegistered('get_window', { window_id: 19 }, context);
        const after = await embedded.registry.executeRegistered('screenshot', {
            target_window_id: 19, quality: 0, width: 480,
        }, context);
        assert.deepEqual(after.content, original.content);
        assert.deepEqual(effects, [['escape', 1]]);
        assert.equal(f.history().history.reservedActions, 57);
    } finally {
        await embedded?.server.close(); f.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
    }
});

test('uncertain Escape remains latched across a new provider, source hash and E2 menu admission', async () => {
    const f = campaignFixture(), names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof createFiniteConsentMcp> | undefined} */ let embedded;
    let nativeAttempts = 0;
    try {
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        const h = openCampaign(f, () => { nativeAttempts++; throw new Error('SYNTHETIC_UNCERTAIN_NATIVE_RESULT'); });
        embedded = h.embedded;
        await freshBefore(embedded, h.context);
        const result = await embedded.registry.executeRegistered('key', escapeArgs, h.context);
        assert.equal(result.isError, true);
        assert.equal(nativeAttempts, 1);
        assert.equal(f.history().uncertain, true);
        assert.equal(f.history().history.reservedActions, 57);
        assert.equal(f.history().history.sentActions, 55);
        await embedded.server.close(); embedded = undefined;

        // A current independently described menu cannot repair the unresolved key.
        f.plan.currentMenuAdmission = { state: 'ORDINARY_GAME_MENU', originalPngSha256: sha(
            fileURLToPath(new URL('./capture/fixtures/readiness.synthetic.png', import.meta.url))) };
        const planPath = resolve(f.scenario, 'plan.json'); write(planPath, f.plan);
        f.lease.planSha256 = sha(planPath); write(f.leasePath, f.lease);
        const e2Owned = resolve(f.scenario, 'driver-E2-owned.private.json');
        write(e2Owned, JSON.parse(readFileSync(f.auth.ownedReceiptPath, 'utf8')));
        const e2 = resolve(f.scenario, 'E2-authorization.private.json');
        write(e2, { ...f.auth, batch: 'E2', planSha256: sha(planPath), leaseSha256: sha(f.leasePath),
            ownedReceiptPath: e2Owned, ownedReceiptSha256: sha(e2Owned) });
        assert.notEqual(sha(e2), sha(f.source));
        assert.throws(() => readBatchAuthorization(e2, { root: f.root, runID: 'fresh-provider-after-uncertainty' }),
            /uncertain|unresolved|latched/i);
        assert.equal(nativeAttempts, 1);
        assert.equal(f.history().uncertain, true);
        assert.equal(f.history().history.reservedActions, 57);
    } finally {
        await embedded?.server.close(); f.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
    }
});

test('the unchanged V1 menu contract refuses key before any delegated effect', async () => {
    let effects = 0;
    const guard = createBatchSession({ dispatch: async () => { effects++; return json({ ok: true }); } }, {
        schemaVersion: 1, batch: 'B1', scenario: 'synthetic-v1', runID: 'v1-key-refusal',
        planSha256: 'a'.repeat(64), ownedPids: [101],
    });
    await assert.rejects(guard.dispatch('key', escapeArgs), /mutation outside remaining sequence/);
    assert.equal(effects, 0);
});
