// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { PassThrough } from 'node:stream';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { Client } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { serveDirectMcp } from './direct-mcp.mjs';
import { campaignHttpFixture } from './campaign-http-test-fixture.mjs';
import { directFixture, syntheticDispatch } from './direct-test-fixture.mjs';
import { escapeArgs } from './campaign-test-fixture.mjs';

/** @param {() => Promise<void>} action */
async function consenting(action) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    try { await action(); }
    finally { names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; }); }
}

test('finite standard stdio forwards one V2 E1 Escape through HTTP with actual outer and inner IDs', async () => consenting(async () => {
    const f = await campaignHttpFixture(), input = new PassThrough(), output = new PassThrough();
    const frontend = serveDirectMcp({ ...f.provider.options, spawn: f.provider.spawn, stdin: input, stdout: output });
    const client = new Client({ name: 'campaign-key-stdio', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const transport = new StdioServerTransport(output, input);
    /** @type {string | number | undefined} */ let outerKeyID;
    const send = transport.send.bind(transport);
    transport.send = async (/** @type {import('../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs').JSONRPCMessage} */ message) => {
        if ('method' in message && message.method === 'tools/call' && message.params?.name === 'key' && 'id' in message) outerKeyID = message.id;
        return send(message);
    };
    try {
        await client.connect(transport);
        assert.notEqual((await client.callTool({ name: 'begin_batch', arguments: {
            source: f.provider.options.source, runID: f.provider.options.runID, consentMode: 'finite-explicit-consent' } })).isError, true);
        /** @type {[string, Record<string, unknown>][]} */
        const before = [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
            ['screenshot', { target_window_id: 19, width: 480, quality: 0 }], ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]];
        for (const [name, args] of before) {
            const result = await client.callTool({ name, arguments: args });
            assert.notEqual(result.isError, true, name + ': ' + JSON.stringify(result));
        }
        const result = await client.callTool({ name: 'key', arguments: escapeArgs });
        assert.notEqual(result.isError, true); assert.deepEqual(f.keys, [['escape', 1]]);
        const call = JSON.parse(readFileSync(resolve(f.output, 'call-007.private.json'), 'utf8'));
        assert.equal(typeof outerKeyID, 'number'); assert.equal(call.outerProtocolID, outerKeyID);
        assert.equal(typeof call.innerProtocolID, 'number'); assert.equal(call.tool, 'key'); assert.deepEqual(call.args, escapeArgs);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'call-007.raw.private.json'), 'utf8')), result);
        assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
        assert.equal(f.history().uncertain, false); assert.equal(f.provider.starts(), 1);
        input.end(); await frontend.closed;
        const terminal = JSON.parse(readFileSync(resolve(f.output, 'owned-provider-terminal.private.json'), 'utf8'));
        assert.equal(terminal.terminalObserved, true); assert.equal(terminal.cleanupUnknown, false);
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.terminalBoundToChild, true);
    } finally { await client.close(); await frontend.close(); await f.close(); }
}));

for (const version of ['V1', 'E2']) test('advertised stdio key retains ' + version + ' raw authority refusal before native effects', async () => consenting(async () => {
    let nativeCalls = 0;
    const campaign = version === 'E2' ? await campaignHttpFixture({ e2: true }) : undefined;
    const provider = campaign?.provider ?? directFixture({ dispatch: async tool => { nativeCalls++; return syntheticDispatch(tool); } });
    const input = new PassThrough(), output = new PassThrough();
    const frontend = serveDirectMcp({ ...provider.options, spawn: provider.spawn, stdin: input, stdout: output });
    const client = new Client({ name: 'campaign-key-version-' + version, version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    try {
        await client.connect(new StdioServerTransport(output, input));
        assert.notEqual((await client.callTool({ name: 'begin_batch', arguments: { source: provider.options.source, runID: provider.options.runID,
            ...(campaign ? { consentMode: 'finite-explicit-consent' } : {}) } })).isError, true);
        const historyBefore = campaign ? readFileSync(campaign.historyPath) : undefined;
        const result = await client.callTool({ name: 'key', arguments: escapeArgs }); assert.equal(result.isError, true);
        const call = JSON.parse(readFileSync(resolve(provider.options.root, '.tmp/starsector-proxy', provider.options.runID, 'call-001.private.json'), 'utf8'));
        assert.equal(call.tool, 'key'); assert.equal(call.failed, true); assert.match(call.reason, /unsupported raw call/i);
        assert.equal(call.innerProtocolID, 'UNKNOWN'); assert.equal(nativeCalls, 0);
        if (campaign) { assert.ok(readFileSync(campaign.historyPath).equals(/** @type {Buffer} */ (historyBefore))); assert.deepEqual(campaign.keys, []); }
        assert.equal(provider.starts(), 1);
    } finally { await client.close(); await frontend.close(); if (campaign) await campaign.close(); else await provider.close(); }
}));
