// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { PassThrough } from 'node:stream';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { Client, InMemoryTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { createDirectMcp, serveDirectMcp } from './direct-mcp.mjs';
import { directToolNames } from './owned-http-batch.mjs';
import { directFixture, syntheticDoctor, syntheticPng, syntheticDispatch } from './direct-test-fixture.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';

test('finite frontend advertises exact vendor schemas/metadata and only two additional controls', async () => {
    const f = directFixture(), frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-catalog-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const embedded = createGuardedMcp({ schemaVersion: 1, batch: 'B1', runID: 'catalog', scenario: 'synthetic', planSha256: 'a'.repeat(64), ownedPids: [101] }, { session: { dispatch: async () => ({ content: [] }) } });
    const [vendorRemote, vendorLocal] = InMemoryTransport.createLinkedPair();
    const vendor = new Client({ name: 'vendor-catalog-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote); await embedded.server.connect(vendorLocal); await vendor.connect(vendorRemote);
        const actual = (await client.listTools()).tools, expected = (await vendor.listTools()).tools;
        assert.deepEqual(actual.map(tool => tool.name).sort(), [...directToolNames, 'key', 'begin_batch', 'finish_batch'].sort());
        for (const tool of actual.filter(tool => directToolNames.includes(tool.name) || tool.name === 'key')) {
            const original = expected.find(row => row.name === tool.name);
            assert.ok(original);
            assert.deepEqual(tool.inputSchema, original.inputSchema, tool.name + ' input');
            assert.deepEqual(tool.outputSchema, original.outputSchema, tool.name + ' output');
            assert.deepEqual(tool.annotations, original.annotations, tool.name + ' annotations');
            assert.deepEqual(tool._meta, original._meta, tool.name + ' metadata');
        }
        assert.equal(f.starts(), 0);
    } finally { await client.close(); await vendor.close(); await frontend.close(); await embedded.server.close(); await f.close(); }
});

test('standard stdio returns real raw results and EOF awaits the owned terminal receipt', async () => {
    const f = directFixture(), input = new PassThrough(), output = new PassThrough();
    const frontend = serveDirectMcp({ ...f.options, spawn: f.spawn, stdin: input, stdout: output });
    const client = new Client({ name: 'finite-stdio-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const transport = new StdioServerTransport(output, input);
    let doctorID;
    const originalSend = transport.send.bind(transport);
    transport.send = async (/** @type {import('../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs').JSONRPCMessage} */ message) => { if ('method' in message && message.method === 'tools/call' && message.params?.name === 'doctor' && 'id' in message) doctorID = message.id; return originalSend(message); };
    try {
        await client.connect(transport);
        const begin = await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        assert.notEqual(begin.isError, true);
        const result = await client.callTool({ name: 'doctor', arguments: {} });
        assert.deepEqual(result.content, [{ type: 'text', text: JSON.stringify(syntheticDoctor) }]);
        assert.deepEqual(result.structuredContent, syntheticDoctor);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'call-001.raw.private.json'), 'utf8')), result);
        const call = JSON.parse(readFileSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'call-001.private.json'), 'utf8'));
        assert.equal(call.outerProtocolID, doctorID); assert.equal(typeof call.innerProtocolID, 'number');
        input.end(); await frontend.closed;
        assert.equal(f.starts(), 1); assert.equal(f.kills(), 0);
    } finally { await client.close(); await frontend.close(); await f.close(); }
});

test('original screenshot is durable before its complete stdio result is emitted', async () => {
    const f = directFixture(), frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-image-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        for (const name of ['doctor', 'policy_status']) await client.callTool({ name, arguments: {} });
        await client.callTool({ name: 'get_window', arguments: { window_id: 19 } });
        const result = await client.callTool({ name: 'screenshot', arguments: { target_window_id: 19, width: 1, quality: 0 } });
        assert.deepEqual(result, { content: [{ type: 'image', mimeType: 'image/png', data: syntheticPng }] });
        const output = resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID);
        assert.deepEqual(readFileSync(resolve(output, 'capture-001.png')), Buffer.from(syntheticPng, 'base64'));
        assert.deepEqual(JSON.parse(readFileSync(resolve(output, 'capture-001.raw.private.json'), 'utf8')), result);
        const finish = await client.callTool({ name: 'finish_batch', arguments: {} }); assert.notEqual(finish.isError, true);
    } finally { await client.close(); await frontend.close(); await f.close(); }
});

test('native error result remains unchanged and failed actual verifier status stays failed', async () => {
    const failure = { content: [{ type: 'text', text: 'Synthetic native refusal' }], isError: true };
    const f = directFixture({ terminalStatus: 1, dispatch: async tool => tool === 'wait' ? failure : syntheticDispatch(tool) });
    const frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-native-error-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        assert.deepEqual(await client.callTool({ name: 'wait', arguments: { duration: 0.1 } }), failure);
        const finish = await client.callTool({ name: 'finish_batch', arguments: {} }); assert.equal(finish.isError, true);
        const receipt = JSON.parse(/** @type {any} */ (finish.content[0]).text);
        assert.equal(receipt.verifierStatus, 1); assert.equal(receipt.steps[0].status, 1); assert.equal(receipt.steps[0].timedOut, false);
        await assert.rejects(frontend.close(), /cleanup/);
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});

test('EOF during startup waits for owned verifier cleanup and never publishes an active batch', async () => {
    const f = directFixture({ neverReady: true }), input = new PassThrough(), output = new PassThrough();
    const frontend = serveDirectMcp({ ...f.options, spawn: f.spawn, stdin: input, stdout: output });
    const client = new Client({ name: 'finite-eof-startup-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await client.connect(new StdioServerTransport(output, input));
        const pending = client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        void pending.catch(() => {}); await new Promise(done => setTimeout(done, 150));
        input.end(); await assert.rejects(frontend.closed, /startup|cleanup/);
        assert.equal(f.kills(), 1);
        assert.ok(existsSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider-terminal.private.json')));
        await client.close(); await pending.catch(() => {});
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});

test('finish holds the exclusive batch lane until terminal; overlap, repeated finish and stopped calls refuse', async () => {
    const f = directFixture({ terminalDelayMs: 300 }), frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-retirement-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        const begin = { name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } };
        await client.callTool(begin);
        const finishing = client.callTool({ name: 'finish_batch', arguments: {} });
        await new Promise(done => setTimeout(done, 40));
        const overlap = await client.callTool(begin);
        assert.equal(overlap.isError, true);
        assert.match(/** @type {any} */ (overlap.content[0]).text, /One qualified finite batch may be active/);
        const stoppedCall = await client.callTool({ name: 'doctor', arguments: {} }); assert.equal(stoppedCall.isError, true);
        assert.notEqual((await finishing).isError, true); assert.equal(f.starts(), 1);
        assert.equal((await client.callTool({ name: 'finish_batch', arguments: {} })).isError, true);
        assert.equal((await client.callTool({ name: 'doctor', arguments: {} })).isError, true);
    } finally { await client.close(); await frontend.close(); await f.close(); }
});

test('concurrent raw requests retire the owned batch and refuse all later input', async () => {
    /** @type {() => void} */ let start = () => { throw new Error('Deferred native boundary not initialized'); };
    /** @type {Promise<void>} */ const started = new Promise(done => { start = done; }); let cancelled = false, dispatches = 0;
    const f = directFixture({ dispatch: async (tool, args, signal) => {
        dispatches++; start();
        return new Promise((done, fail) => { signal?.addEventListener('abort', () => { cancelled = true; fail(new Error('Synthetic native aborted')); }, { once: true }); });
    } });
    const frontend = createDirectMcp({ ...f.options, spawn: f.spawn }); const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-overlap-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        const begin = { name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } };
        await client.callTool(begin);
        const first = client.callTool({ name: 'wait', arguments: { duration: 0.1 } });
        await started;
        assert.equal((await client.callTool({ name: 'doctor', arguments: {} })).isError, true);
        assert.equal((await first).isError, true); assert.equal(cancelled, true); assert.equal(dispatches, 1);
        assert.equal((await client.callTool({ name: 'left_click', arguments: { coordinate: [10, 10], target_window_id: 19, focus_strategy: 'strict' } })).isError, true);
        await client.callTool({ name: 'finish_batch', arguments: {} });
        assert.equal((await client.callTool(begin)).isError, true); assert.equal(f.starts(), 1);
    } finally { await client.close(); await frontend.close(); await f.close(); }
});

test('unsupported approval_token is never forwarded or persisted in any private artifact', async () => {
    const marker = 'unsupported-private-token-marker-7721'; let dispatches = 0;
    const f = directFixture({ dispatch: async tool => { dispatches++; return syntheticDispatch(tool); } });
    const frontend = createDirectMcp({ ...f.options, spawn: f.spawn }); const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-token-privacy-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        const refused = await client.callTool({ name: 'left_click', arguments: { coordinate: [10, 10], target_window_id: 19, focus_strategy: 'strict', approval_token: marker } });
        assert.equal(refused.isError, true); assert.equal(dispatches, 0); assert.equal(JSON.stringify(refused).includes(marker), false);
        const output = resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID);
        for (const entry of readdirSync(output, { recursive: true, withFileTypes: true })) if (entry.isFile())
            assert.equal(readFileSync(resolve(entry.parentPath, entry.name), 'utf8').includes(marker), false, entry.name);
    } finally { await client.close(); await frontend.close(); await f.close(); }
});

test('missing actual verifier close remains UNKNOWN and poisons new batch admission', { timeout: 10000 }, async () => {
    const f = directFixture({ terminalNever: true, killReturnsFalse: true });
    const frontend = createDirectMcp({ ...f.options, spawn: f.spawn }); const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'finite-unknown-reap-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        const begin = { name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } }; await client.callTool(begin);
        const finished = await client.callTool({ name: 'finish_batch', arguments: {} }); assert.equal(finished.isError, true);
        const receipt = JSON.parse(/** @type {any} */ (finished.content[0]).text);
        assert.equal(receipt.terminalObserved, false); assert.equal(receipt.cleanupUnknown, true); assert.equal(receipt.verifierStatus, null);
        assert.deepEqual(receipt.steps, []); assert.equal(f.kills(), 1);
        assert.equal((await client.callTool(begin)).isError, true); assert.equal(f.starts(), 1);
        await assert.rejects(frontend.close(), /cleanup/);
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});
