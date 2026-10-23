// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { PassThrough } from 'node:stream';
import { writeFileSync, readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Client, InMemoryTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { createDirectMcp, serveDirectMcp } from './direct-mcp.mjs';
import { directFixture } from './direct-test-fixture.mjs';

test('unknown startup cleanup propagates its receipt and latches admission of a different valid batch', { timeout: 15000 }, async () => {
    const f = directFixture({ wrongParent: true, terminalNever: true, killReturnsFalse: true });
    const frontend = createDirectMcp({ ...f.options, spawn: f.spawn }), [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'startup-unknown-lifecycle-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        const failed = await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        assert.equal(failed.isError, true);
        const next = await client.callTool({ name: 'begin_batch', arguments: f.next }); assert.equal(next.isError, true);
        assert.equal(f.starts(), 1); assert.equal(existsSync(resolve(f.scenario, 'guard-B2-consumed.private.json')), false);
        const outcome = JSON.parse(/** @type {any} */ (failed.content[0]).text);
        assert.equal(outcome.status, 'STARTUP_FAILED'); assert.equal(outcome.cleanupReceipt.cleanupUnknown, true);
        await assert.rejects(frontend.close(), error => /** @type {any} */ (error).cleanupReceipt?.cleanupUnknown === true);
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});

test('throwing finish cleanup preserves its artifact, latches the frontend and propagates shutdown failure', async () => {
    const f = directFixture(), frontend = createDirectMcp({ ...f.options, spawn: f.spawn }), [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'throwing-finish-lifecycle-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const marker = 'preserve-existing-spool';
    try {
        await frontend.server.connect(local); await client.connect(remote);
        await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        const spool = resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider.stdout.private.txt'); writeFileSync(spool, marker);
        assert.equal((await client.callTool({ name: 'finish_batch', arguments: {} })).isError, true);
        assert.equal((await client.callTool({ name: 'begin_batch', arguments: f.next })).isError, true);
        assert.equal(f.starts(), 1); assert.equal(readFileSync(spool, 'utf8'), marker);
        await assert.rejects(frontend.close(), /cleanup failed/);
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});

for (const fault of ['unknown', 'throwing']) test('EOF rejects its closed receipt after ' + fault + ' owned cleanup', { timeout: 10000 }, async () => {
    const f = directFixture(fault === 'unknown' ? { terminalNever: true, killReturnsFalse: true } : {});
    const input = new PassThrough(), output = new PassThrough();
    const frontend = serveDirectMcp({ ...f.options, spawn: f.spawn, stdin: input, stdout: output });
    const client = new Client({ name: 'eof-failed-lifecycle-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await client.connect(new StdioServerTransport(output, input));
        await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } });
        if (fault === 'throwing') writeFileSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider.stdout.private.txt'), 'retained');
        input.end(); await assert.rejects(frontend.closed, /cleanup/);
        if (fault === 'unknown') {
            const receipt = JSON.parse(readFileSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider-terminal.private.json'), 'utf8'));
            assert.equal(receipt.cleanupUnknown, true); assert.equal(receipt.terminalObserved, false);
        }
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});
