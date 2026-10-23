// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { PassThrough } from 'node:stream';
import { Client } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { serveComputerUseStdio } from './vendor-runtime.mjs';
import { createCaptureCustody } from './capture-custody.mjs';
import { custodyFixture, expectedImage, screenshotArgs, png } from './capture-custody-fixture.mjs';
test('standard raw MCP reply waits for exact declared artifacts and callback completion proof', { timeout: 10000 }, async () => {
    const f = custodyFixture(), input = new PassThrough(), output = new PassThrough();
    const custody = createCaptureCustody({ admitted: f.admitted, readIntent: f.readIntent, now: f.now });
    let dispatches = 0, actualRequestID;
    const handle = serveComputerUseStdio({ profile: 'core', activeProfile: 'core', ...custody,
        session: /** @type {any} */ ({ dispatch: async () => { dispatches++; return expectedImage; } }) },
        { transport: new StdioServerTransport(input, output) });
    const client = new Client({ name: 'capture-custody-completion-test', version: '1' },
        { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
    const transport = new StdioServerTransport(output, input);
    const send = transport.send.bind(transport);
    transport.send = async message => {
        if ('method' in message && message.method === 'tools/call' && message.params?.name === 'screenshot' && 'id' in message)
            actualRequestID = message.id;
        return send(message);
    };
    try {
        await client.connect(transport);
        const result = await client.callTool({ name: 'screenshot', arguments: screenshotArgs });
        assert.ok(existsSync(f.slot.callbackReceipt), 'completion must exist before the original MCP reply');
        assert.deepEqual(result, expectedImage);
        assert.equal(dispatches, 1);
        assert.equal(f.reads(), 1, 'freeze the exact intent once before dispatch');
        assert.deepEqual(readFileSync(f.slot.png), png);
        assert.equal(readFileSync(f.slot.payload, 'utf8'), png.toString('base64') + '\n');
        const proof = JSON.parse(readFileSync(f.slot.callbackReceipt, 'utf8'));
        assert.equal(proof.kind, 'NATIVE99_CAPTURE_CALLBACK_COMPLETION_V1');
        assert.equal(proof.status, 'PERSISTED_ORIGINAL_RECEIPT_ONLY');
        assert.equal(proof.protocolRequestID, actualRequestID);
        assert.equal(proof.intentSha256, createHash('sha256').update(readFileSync(f.intentPath)).digest('hex'));
        assert.equal(proof.png, f.slot.png);
        assert.equal(proof.bytes, png.length);
        assert.equal(proof.width, 480); assert.equal(proof.height, 220);
        assert.equal(proof.freshness, 'UNKNOWN');
        assert.equal(proof.originalPayloadRoundtripExact, true);
        assert.equal(proof.ownershipEvidence, 'CALLER_ORIGIN_CURRENT_WITNESS_NOT_INDEPENDENT_HOST_ATTESTATION');
        assert.deepEqual(proof.sourcePins, f.admitted.sourcePins);
        assert.deepEqual(JSON.parse(readFileSync(proof.rawPath, 'utf8')), expectedImage);
    } finally { await client.close(); await handle.close(); input.destroy(); output.destroy(); }
});
