// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { randomBytes } from 'node:crypto';
import { request } from 'node:http';
import { Client, StreamableHTTPClientTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';
import { serveGuardedHttp } from './guarded-http.mjs';

const binding = { schemaVersion: 1, batch: 'B1', runID: 'http-synthetic', scenario: 'synthetic',
    planSha256: 'b'.repeat(64), ownedPids: [202] };
const target = { pid: 202, windowId: 19, title: 'Starsector', bundleId: 'java.exe', isFocused: true,
    bounds: { x: 10, y: 10, width: 100, height: 80 } };
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=';
/** @param {any} value */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
const noReconnect = { maxRetries: 0, maxReconnectionDelay: 0, initialReconnectionDelay: 0, reconnectionDelayGrowFactor: 1 };

function deferred() {
    /** @type {() => void} */ let release = () => { throw new Error('Deferred is not initialized'); };
    /** @type {Promise<void>} */ const promise = new Promise(done => { release = done; });
    return { promise, release };
}

/** Use the Node HTTP boundary for hostile Host headers; fetch normalizes them.
 * @param {URL} url @param {Record<string, string | undefined>} headers @param {string} body */
async function post(url, headers, body) {
    return new Promise((resolve, reject) => {
        const req = request(url, { method: 'POST', headers }, res => {
            res.resume(); res.once('end', () => resolve(res.statusCode));
        });
        req.once('error', reject); req.end(body);
    });
}

/** Real pinned registry/guard/SDK with only native dispatch replaced.
 * @param {(tool: string, args: any, signal?: AbortSignal) => Promise<any>} [override]
 * @param {{connect?: boolean}} [options] */
async function fixture(override = async () => undefined, { connect = true } = {}) {
    let effects = 0;
    /** @type {Record<string, unknown>[]} */ const events = [];
    const embedded = createGuardedMcp(binding, { record: event => events.push(event), session: {
        dispatch: async (/** @type {string} */ tool, /** @type {any} */ args, /** @type {AbortSignal} */ signal) => {
            const changed = await override(tool, args, signal);
            if (changed !== undefined) return changed;
            if (tool === 'doctor') return json({ ok: true });
            if (tool === 'policy_status') return json({ profile: 'core' });
            if (tool === 'get_window') return json(target);
            if (tool === 'get_frontmost_app') return json({ app: { pid: 202, bundleId: 'java.exe', displayName: 'Synthetic' } });
            if (tool === 'screenshot') return { content: [{ type: 'image', mimeType: 'image/png', data: png }] };
            if (tool === 'left_click') { effects++; return json({ clicked: true }); }
            return json({ windows: [target] });
        }, getLastScreenshot: () => ({ mimeType: 'image/png', data: png, capturedAt: 123 }),
    } });
    const token = randomBytes(32).toString('hex');
    const host = await serveGuardedHttp({ embedded, token });
    const transport = new StreamableHTTPClientTransport(new URL(host.url), {
        requestInit: { headers: { Authorization: 'Bearer ' + token } },
        reconnectionOptions: noReconnect,
    });
    const client = new Client({ name: 'synthetic-driver', version: '1' }, {
        capabilities: {}, supportedProtocolVersions: ['2025-11-25'],
    });
    try { if (connect) await client.connect(transport); }
    catch (error) { await client.close(); await host.close(); throw error; }
    return { host, embedded, client, transport, token, events, effects: () => effects,
        close: async () => { try { await transport.terminateSession(); } finally { await client.close(); await host.close(); } } };
}

test('separate HTTP calls retain one finite guard and the original image', { timeout: 10000 }, async () => {
    const f = await fixture();
    /** @param {string} name @param {any} [args] */
    const call = (name, args = {}) => f.client.callTool({ name, arguments: args });
    try {
        await call('doctor'); await call('policy_status');
        await call('get_window', { window_id: 19 });
        const image = await call('screenshot', { target_window_id: 19, quality: 0, width: 100 });
        assert.deepEqual(image.content.find(item => item.type === 'image'), { type: 'image', mimeType: 'image/png', data: png });
        await call('get_frontmost_app'); await call('get_window', { window_id: 19 });
        const click = { target_window_id: 19, focus_strategy: 'strict', coordinate: [20, 20] };
        const first = await call('left_click', click);
        assert.notEqual(first.isError, true); assert.equal(f.effects(), 1);
        await assert.rejects(call('left_click', click), /outside remaining sequence/);
        assert.equal(f.effects(), 1);
        assert.equal(f.events.filter(row => row.phase === 'before-dispatch' && row.tool === 'left_click').length, 1);
        assert.ok(f.transport.sessionId);
    } finally { await f.close(); }
});

test('HTTP admission rejects missing bearer, foreign origins, paths and unsupported protocol before dispatch', { timeout: 10000 }, async () => {
    const f = await fixture();
    const body = { jsonrpc: '2.0', id: 800, method: 'tools/call', params: { name: 'doctor', arguments: {} } };
    const headers = { Authorization: 'Bearer ' + f.token, 'Content-Type': 'application/json',
        Accept: 'application/json, text/event-stream', 'mcp-session-id': /** @type {string} */ (f.transport.sessionId),
        'mcp-protocol-version': '2025-11-25' };
    try {
        for (const entry of [
            { status: 401, headers: { Authorization: '' } },
            { status: 401, headers: { Authorization: 'Bearer ' + '0'.repeat(64) } },
            { status: 403, headers: { Origin: 'https://example.invalid' } },
            { status: 403, headers: { Host: 'example.invalid' } },
            { status: 400, headers: { 'mcp-protocol-version': '2026-07-28' } },
            { status: 404, path: '/other' }, { status: 404, path: '/mcp?token=forbidden' },
            { status: 400, body: '{' }, { status: 400, body: '[]' },
            { status: 400, body: JSON.stringify({ ...body, method: 'tasks/get' }) },
            { status: 400, body: JSON.stringify({ ...body, params: { ...body.params,
                _meta: { 'io.modelcontextprotocol/protocolVersion': '2026-07-28' } } }) },
            { status: 413, body: ' '.repeat(65537) },
        ]) {
            const url = new URL(f.host.url); if (entry.path) { const parsed = new URL(entry.path, url); url.pathname = parsed.pathname; url.search = parsed.search; }
            const status = await post(url, { ...headers, ...entry.headers }, entry.body ?? JSON.stringify(body));
            assert.equal(status, entry.status, JSON.stringify(entry));
        }
        assert.equal(f.events.length, 0); assert.equal(f.effects(), 0);
    } finally { await f.close(); }
});

test('legacy cancellation reaches the existing guarded native dispatch', { timeout: 10000 }, async () => {
    const started = deferred(), stopped = deferred();
    const f = await fixture(async (tool, args, signal) => {
        if (tool !== 'list_windows') return;
        started.release();
        return new Promise((resolve, reject) => {
            signal?.addEventListener('abort', () => { stopped.release(); reject(new Error('Synthetic dispatch cancelled')); }, { once: true });
        });
    });
    try {
        const controller = new AbortController();
        const pending = f.client.callTool({ name: 'list_windows', arguments: {} }, { signal: controller.signal });
        const rejected = assert.rejects(pending);
        await started.promise; controller.abort(); await rejected; await stopped.promise;
        assert.equal(f.effects(), 0);
    } finally { await f.close(); }
});

test('a second client and an incorrect session cannot recreate batch authority', { timeout: 10000 }, async () => {
    const f = await fixture();
    const client = new Client({ name: 'other', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const transport = new StreamableHTTPClientTransport(new URL(f.host.url), {
        requestInit: { headers: { Authorization: 'Bearer ' + f.token } }, reconnectionOptions: noReconnect,
    });
    try {
        await assert.rejects(client.connect(transport));
        for (const id of ['', 'wrong-session']) {
            const response = await fetch(f.host.url, { method: 'POST', headers: { Authorization: 'Bearer ' + f.token,
                'Content-Type': 'application/json', Accept: 'application/json, text/event-stream',
                ...(id ? { 'mcp-session-id': id } : {}) },
            body: JSON.stringify({ jsonrpc: '2.0', id: 900, method: 'tools/call', params: { name: 'doctor', arguments: {} } }) });
            assert.equal(response.status, id ? 404 : 400); await response.text();
        }
        assert.equal(f.events.length, 0);
        const result = await f.client.callTool({ name: 'doctor', arguments: {} });
        assert.notEqual(result.isError, true);
    } finally { await client.close(); await f.close(); }
});

test('policy failure preserves complete CallToolResult rather than claiming success', { timeout: 10000 }, async () => {
    const denied = { isError: true, structuredContent: { error: 'approval_required', reasons: ['approval_required_for_all'] },
        content: [{ type: 'text', text: 'approval_required' }] };
    const f = await fixture(async tool => tool === 'doctor' ? denied : undefined);
    try {
        const result = await f.client.callTool({ name: 'doctor', arguments: {} });
        assert.equal(result.isError, true); assert.deepEqual(result.content, denied.content);
        assert.deepEqual(result.structuredContent, denied.structuredContent); assert.equal(f.effects(), 0);
    } finally { await f.close(); }
});

test('concurrent HTTP calls retain request-overlap refusal and input latch', { timeout: 10000 }, async () => {
    const started = deferred(), held = deferred();
    const f = await fixture(async tool => { if (tool === 'list_windows') { started.release(); await held.promise; } });
    try {
        const first = f.client.callTool({ name: 'list_windows', arguments: {} });
        await started.promise;
        await assert.rejects(f.client.callTool({ name: 'doctor', arguments: {} }), /Concurrent Computer Use request refused/);
        held.release(); await first;
        assert.ok(f.events.some(row => row.phase === 'request-overlap'));
        await assert.rejects(f.client.callTool({ name: 'left_click', arguments: {
            target_window_id: 19, focus_strategy: 'strict', coordinate: [20, 20],
        } }), /latched/);
        assert.equal(f.effects(), 0);
    } finally { held.release(); await f.close(); }
});

test('unsupported legacy client capabilities refuse before guard dispatch or continuation', { timeout: 10000 }, async () => {
    for (const capabilities of [{ roots: { listChanged: true } }, { elicitation: {} }, { sampling: {} },
        { extensions: { 'io.modelcontextprotocol/tasks': {} } }, [], null]) {
        const f = await fixture(undefined, { connect: false });
        try {
            const response = await post(new URL(f.host.url), { Authorization: 'Bearer ' + f.token,
                'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' },
            JSON.stringify({ jsonrpc: '2.0', id: 1000, method: 'initialize', params: {
                protocolVersion: '2025-11-25', capabilities, clientInfo: { name: 'unsupported', version: '1' },
            } }));
            assert.equal(response, 400); assert.equal(f.events.length, 0);
            await f.client.connect(f.transport);
            assert.ok(f.transport.sessionId);
        } finally { await f.close(); }
    }
});

test('shutdown failures remain failures through both close receipts', { timeout: 10000 }, async () => {
    const f = await fixture();
    const originalClose = f.embedded.server.close.bind(f.embedded.server);
    f.embedded.server.close = async () => { await originalClose(); throw new Error('Synthetic guard close failure'); };
    try {
        await assert.rejects(f.host.close(), /Synthetic guard close failure/);
        await assert.rejects(f.host.closed, /Synthetic guard close failure/);
        await assert.rejects(fetch(f.host.url));
    } finally { await f.client.close(); await f.host.close().catch(() => {}); }
});

test('explicit session termination closes the listener and existing guard', { timeout: 10000 }, async () => {
    const f = await fixture();
    try {
        await f.transport.terminateSession(); await f.host.closed;
        await assert.rejects(fetch(f.host.url));
        await f.host.close();
    } finally { await f.client.close(); await f.host.close(); }
});
