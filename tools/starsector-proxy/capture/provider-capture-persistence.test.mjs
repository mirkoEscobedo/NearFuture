import { makeOutput, testOutputRoot } from './test-output.mjs';
// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client, InMemoryTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createComputerUseServer } from './vendor-runtime.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';

const base = new URL('.', import.meta.url);
const fixture = readFileSync(new URL('./fixtures/readiness.synthetic.png', import.meta.url));
const expected = { content: [{ type: 'image', mimeType: 'image/png', data: fixture.toString('base64') }] };
const args = { target_window_id: 19, width: 480, quality: 0 };
const window = { pid: 101, windowId: 19, bounds: { width: 480, height: 220 } };

test('factory awaits original screenshot custody before returning the MCP result', { timeout: 10000 }, async () => {
    const output = makeOutput('synthetic-custody-');
    let release = () => {};
    const gate = new Promise(done => { release = () => done(undefined); });
    let entered = () => {};
    const observing = new Promise(done => { entered = () => done('observer'); });
    let calls = 0, completed = false;
    const server = createComputerUseServer({
        profile: 'core', activeProfile: 'core',
        session: /** @type {any} */ ({ dispatch: async () => expected }),
        onToolResult: async (/** @type {any} */ context) => {
            calls++;
            assert.equal(context.definition.name, 'screenshot');
            assert.deepEqual(context.args, args);
            assert.deepEqual(context.result, expected);
            entered();
            await gate;
            persistDirectFrame({ output, label: 'capture-001', result: context.result, args: context.args,
                window, ownedPids: [101], provenance: { kind: 'SYNTHETIC_ONLY', native: false } });
        },
    });
    const client = new Client({ name: 'capture-custody-test', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    /** @type {Promise<any>|undefined} */ let pending;
    try {
        await server.connect(local);
        await client.connect(remote);
        pending = client.callTool({ name: 'screenshot', arguments: args }).then(result => { completed = true; return result; });
        const first = await Promise.race([observing, pending.then(() => 'reply')]);
        assert.equal(first, 'observer', 'the public factory must call its observer before replying');
        assert.equal(completed, false, 'custody completion must be awaited');
        assert.equal(existsSync(resolve(output, 'capture-001.png')), false);
        release();
        const actual = await pending;
        assert.equal(calls, 1);
        assert.deepEqual(actual, expected);
        assert.deepEqual(readFileSync(resolve(output, 'capture-001.png')), fixture);
        assert.deepEqual(JSON.parse(readFileSync(resolve(output, 'capture-001.raw.private.json'), 'utf8')), expected);
        const metadata = JSON.parse(readFileSync(resolve(output, 'capture-001.metadata.private.json'), 'utf8'));
        assert.equal(metadata.captureReceipt, true);
        assert.equal(metadata.freshness, 'UNKNOWN');
        assert.equal(metadata.native, false);
    } finally {
        release();
        await pending?.catch(() => {});
        await client.close();
        await server.close();
    }
});

test('standard stdio embedding forwards custody observer and retains the vendor tool surface', { timeout: 10000 }, async () => {
    const candidate = await import('./vendor-runtime.mjs');
    assert.equal(typeof candidate.serveComputerUseStdio, 'function',
        'the standard vendor stdio route must accept embedding options');
    const { PassThrough } = await import('node:stream');
    const { StdioServerTransport } = await import('../../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs');
    const { createComputerUseServer: originalFactory } = await import('../../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/server.js');
    const input = new PassThrough(), output = new PassThrough();
    const artifacts = makeOutput('synthetic-stdio-');
    let observed = 0;
    const handle = candidate.serveComputerUseStdio({
        profile: 'core', activeProfile: 'core',
        session: /** @type {any} */ ({ dispatch: async () => expected }),
        onToolResult: (/** @type {any} */ context) => {
            observed++;
            persistDirectFrame({ output: artifacts, label: 'capture-001', result: context.result, args: context.args,
                window, ownedPids: [101], provenance: { kind: 'SYNTHETIC_STDIO_ONLY', native: false } });
        },
    }, { transport: new StdioServerTransport(input, output) });
    const client = new Client({ name: 'capture-stdio-test', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const original = originalFactory({ profile: 'core', activeProfile: 'core',
        session: /** @type {any} */ ({ dispatch: async () => expected }) });
    const baseline = new Client({ name: 'capture-catalog-baseline', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    try {
        await original.connect(local); await baseline.connect(remote);
        await client.connect(new StdioServerTransport(output, input));
        assert.deepEqual((await client.listTools()).tools, JSON.parse(JSON.stringify((await baseline.listTools()).tools)));
        const actual = await client.callTool({ name: 'screenshot', arguments: args });
        assert.equal(observed, 1);
        assert.deepEqual(actual, expected);
        assert.deepEqual(readFileSync(resolve(artifacts, 'capture-001.png')), fixture);
        assert.deepEqual(JSON.parse(readFileSync(resolve(artifacts, 'capture-001.raw.private.json'), 'utf8')), expected);
        input.end();
        await handle.close();
    } finally {
        await client.close(); await handle.close();
        await baseline.close(); await original.close();
        input.destroy(); output.destroy();
    }
});

test('observer is optional, cannot replace results by returning a value, and sees native refusals', { timeout: 10000 }, async t => {
    const nativeError = { content: [{ type: 'text', text: 'SYNTHETIC_NATIVE_REFUSAL' }], isError: true };
    for (const [name, result, observer] of [
        ['default', expected, undefined],
        ['ignored return', expected, () => nativeError],
        ['native refusal', nativeError, (/** @type {any} */ context) => assert.deepEqual(context.result, nativeError)],
    ]) await t.test(/** @type {string} */ (name), async () => {
        const server = createComputerUseServer({
            profile: 'core', activeProfile: 'core',
            session: /** @type {any} */ ({ dispatch: async () => result }),
            onToolResult: /** @type {any} */ (observer),
        });
        const client = new Client({ name: 'capture-result-preservation', version: '1' },
            { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
        const [remote, local] = InMemoryTransport.createLinkedPair();
        try {
            await server.connect(local); await client.connect(remote);
            assert.deepEqual(await client.callTool({ name: 'screenshot', arguments: args }), result);
        } finally { await client.close(); await server.close(); }
    });
});

test('observer failure refuses the reply instead of returning an unpreserved image', { timeout: 10000 }, async () => {
    let dispatched = 0;
    const server = createComputerUseServer({
        profile: 'core', activeProfile: 'core',
        session: /** @type {any} */ ({ dispatch: async () => { dispatched++; return expected; } }),
        onToolResult: () => { throw new Error('SYNTHETIC_CUSTODY_FAILURE'); },
    });
    const client = new Client({ name: 'capture-custody-error', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    try {
        await server.connect(local); await client.connect(remote);
        await assert.rejects(client.callTool({ name: 'screenshot', arguments: args }),
            { code: -32603, message: 'SYNTHETIC_CUSTODY_FAILURE' });
        assert.equal(dispatched, 1);
    } finally { await client.close(); await server.close(); }
});
