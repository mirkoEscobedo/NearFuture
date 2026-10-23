// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { CLIENT_CAPABILITIES_META_KEY, PROTOCOL_VERSION_META_KEY } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/index.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';
import { handleWindowTool } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session/window-handlers.js';

const binding = { schemaVersion: 1, batch: 'B1', runID: 'mock-registry', scenario: 'mock-scenario',
    planSha256: 'b'.repeat(64), ownedPids: [202] };
const target = { pid: 202, windowId: 19, title: 'Starsector', bundleId: 'java.exe', isFocused: true,
    bounds: { x: 10, y: 10, width: 100, height: 80 } };
const image = { content: [{ type: 'image', mimeType: 'image/png', data: 'unchanged-original' }] };
/** @param {any} value */ const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
/** @param {boolean} [modern] */
function context(modern = false) {
    return { mcpReq: { method: 'tools/call', signal: new AbortController().signal,
        requestState: () => undefined, inputResponses: {}, log: async () => {}, notify: async () => {},
        envelope: modern ? { [PROTOCOL_VERSION_META_KEY]: '2026-07-28',
            [CLIENT_CAPABILITIES_META_KEY]: { elicitation: { form: {} } } } : {} } };
}
/** @param {(tool: string, args: any, signal?: AbortSignal, progress?: any, ctx?: any) => Promise<any>} [override] */
function mockSession(override = async () => undefined) {
    return { dispatch: async (/** @type {string} */ tool, /** @type {any} */ args,
        /** @type {AbortSignal | undefined} */ signal, /** @type {any} */ progress, /** @type {any} */ ctx) => {
        const value = await override(tool, args, signal, progress, ctx); if (value !== undefined) return value;
        if (tool === 'doctor') return json({ ok: true });
        if (tool === 'policy_status') return json({ profile: 'core' });
        if (tool === 'get_window') return json(target);
        if (tool === 'get_frontmost_app') return handleWindowTool(tool, args, /** @type {any} */ ({ native: { getFrontmostApp: () => ({ pid: 202, bundleId: 'java.exe', displayName: 'Synthetic Starsector' }) } }));
        if (tool === 'screenshot') return image;
        if (tool === 'list_windows') return json({ windows: [target] });
        return json({ windowId: 19, activated: true });
    }, getLastScreenshot: () => ({ mimeType: 'image/png', data: 'unchanged-original', capturedAt: 123 }) };
}
/** @param {any} embedded @param {any} ctx */
async function before(embedded, ctx) {
    for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
        ['screenshot', { target_window_id: 19, quality: 0, width: 100 }], ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) {
        await embedded.registry.executeRegistered(tool, args, ctx);
    }
}

test('actual pinned registry and resource callbacks share one full native dispatch queue', async () => {
    let active = 0, maximum = 0;
    const embedded = createGuardedMcp(binding, { session: mockSession(async () => {
        active++; maximum = Math.max(maximum, active); await new Promise(resolve => setTimeout(resolve, 5)); active--;
    }) });
    try {
        const ctx = context(); await before(embedded, ctx);
        const resource = /** @type {any} */ (embedded.server)._registeredResources['computer://windows'];
        const results = await Promise.allSettled([embedded.registry.executeRegistered('get_frontmost_app', {}, ctx),
            resource.readCallback(new URL('computer://windows'))]);
        assert.equal(maximum, 1); assert.equal(results[0].status, 'fulfilled'); assert.equal(results[1].status, 'rejected');
        const receipt = await resource.readCallback(new URL('computer://windows'));
        assert.equal(JSON.parse(receipt.contents[0].text).windows[0].pid, 202);
        const original = await embedded.registry.executeRegistered('screenshot', { target_window_id: 19, quality: 0, width: 100 }, ctx);
        assert.equal(original.content.find(/** @param {any} item */ item => item.type === 'image').data, 'unchanged-original');
        assert.equal(embedded.registry.activeProfile(), 'core'); assert.equal(embedded.registry.isWithinMaximum('filesystem'), false);
    } finally { await embedded.server.close(); }
});

test('actual registry translates pre-effect modern approval into a non-resumable guard refusal', async () => {
    let effects = 0;
    const embedded = createGuardedMcp(binding, { session: mockSession(async (tool, args, signal, progress, ctx) => {
        if (tool !== 'activate_window') return;
        const approved = await ctx.elicitApproval({ tool, args, reasons: ['approval_required'], destructive: false });
        if (approved) effects++;
        return json({ windowId: 19, activated: approved });
    }) });
    try {
        const ctx = context(true); await before(embedded, ctx);
        await assert.rejects(embedded.registry.executeRegistered('activate_window', { window_id: 19 }, ctx), /UNSUPPORTED_APPROVAL_CONTINUATION/);
        await assert.rejects(embedded.registry.executeRegistered('activate_window', { window_id: 19 }, ctx), /latched/);
        assert.equal(effects, 0); await before(embedded, ctx);
    } finally { await embedded.server.close(); }
});

test('legacy approval is declined before an interaction or native effect and cannot reuse its slot', async () => {
    let effects = 0;
    /** @type {Record<string, unknown>[]} */ const records = [];
    const embedded = createGuardedMcp(binding, { record: event => records.push(event),
        sessionFactory: options => mockSession(async (tool, args) => {
            if (tool !== 'activate_window') return;
            const approved = await options?.elicitApproval?.({ tool, args, reasons: ['approval_required'], destructive: false });
            if (approved) effects++;
            return approved ? json({ windowId: 19, activated: true }) : { isError: true, content: [{ type: 'text', text: 'native policy denied' }] };
        }) });
    try {
        const ctx = context(); await before(embedded, ctx);
        const result = await embedded.registry.executeRegistered('activate_window', { window_id: 19 }, ctx);
        assert.equal(result.isError, true); assert.equal(effects, 0);
        assert.ok(records.some(event => event.phase === 'unsupported-approval-continuation' && event.protocol === 'legacy'));
        await assert.rejects(embedded.registry.executeRegistered('activate_window', { window_id: 19 }, ctx), /latched/);
        await before(embedded, ctx);
    } finally { await embedded.server.close(); }
});
