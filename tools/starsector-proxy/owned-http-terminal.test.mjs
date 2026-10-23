// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, realpathSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Client, InMemoryTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { createDirectMcp } from './direct-mcp.mjs';
import { directFixture } from './direct-test-fixture.mjs';

test('finish accepts actual verifier raw cwd period for the exact private project and child command', async () => {
    const f = directFixture(), frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'actual-terminal-cwd-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        assert.notEqual((await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } })).isError, true);
        const expectedProject = resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider');
        assert.equal(f.args()[1], expectedProject);
        const config = JSON.parse(readFileSync(resolve(expectedProject, '.agentic/project.json'), 'utf8'));
        assert.deepEqual(config.overrides.workspace.commands['node:.'], [{ program: realpathSync.native(process.execPath),
            args: [resolve(f.options.root, 'tools/starsector-proxy/guarded-http-child.mjs')], cwd: '.' }]);
        const result = await client.callTool({ name: 'finish_batch', arguments: {} });
        assert.notEqual(result.isError, true);
        const receipt = JSON.parse(/** @type {any} */ (result.content[0]).text);
        assert.equal(receipt.steps[0].cwd, '.'); assert.equal(receipt.terminalBoundToChild, true);
        assert.equal(receipt.terminalObserved, true); assert.equal(receipt.cleanupUnknown, false);
        assert.equal(f.starts(), 1); assert.equal(f.kills(), 0);
        await frontend.close();
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});

for (const fault of [
    { label: 'parent cwd', terminalStep: { cwd: '..' } },
    { label: 'normalization-equivalent cwd', terminalStep: { cwd: 'nested/..' } },
    { label: 'absolute different project root', terminalStep: { cwd: resolve('synthetic-other-project') } },
    { label: 'different command', terminalStep: { command: 'synthetic-other-node.exe' } },
    { label: 'different child arguments', terminalStep: { args: ['synthetic-other-child.mjs'] } },
]) test('finish refuses ' + fault.label + ' and latches later valid batch admission', async () => {
    const f = directFixture({ terminalStep: fault.terminalStep }), frontend = createDirectMcp({ ...f.options, spawn: f.spawn });
    const [remote, local] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: 'wrong-terminal-binding-test', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await frontend.server.connect(local); await client.connect(remote);
        assert.notEqual((await client.callTool({ name: 'begin_batch', arguments: { source: f.options.source, runID: f.options.runID } })).isError, true);
        const result = await client.callTool({ name: 'finish_batch', arguments: {} }); assert.equal(result.isError, true);
        const receipt = JSON.parse(/** @type {any} */ (result.content[0]).text);
        assert.equal(receipt.terminalObserved, true); assert.equal(receipt.cleanupUnknown, false);
        assert.equal(receipt.verifierStatus, 0); assert.equal(receipt.steps[0].status, 0); assert.equal(receipt.steps[0].timedOut, false);
        assert.equal(receipt.terminalBoundToChild, false);
        for (const [key, value] of Object.entries(fault.terminalStep)) assert.deepEqual(receipt.steps[0][key], value);
        assert.equal((await client.callTool({ name: 'begin_batch', arguments: f.next })).isError, true);
        assert.equal(existsSync(resolve(f.scenario, 'guard-B2-consumed.private.json')), false); assert.equal(f.starts(), 1);
        assert.equal((await client.callTool({ name: 'doctor', arguments: {} })).isError, true);
        await assert.rejects(frontend.close(), /cleanup/);
    } finally { await client.close(); await frontend.close().catch(() => {}); await f.close(); }
});
