// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignHttpFixture } from './campaign-http-test-fixture.mjs';
import { directFixture } from './direct-test-fixture.mjs';
import { escapeArgs, sha } from './campaign-test-fixture.mjs';
import { startOwnedHttpBatch, directToolNames, requiredOwnedHttpFiles } from './owned-http-batch.mjs';

/** @param {() => Promise<void>} action */
async function consenting(action) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    try { await action(); }
    finally { names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; }); }
}

test('owned HTTP V2 E1 sends one exact Escape through finite consent and vendor native dispatch', async () => consenting(async () => {
    const f = await campaignHttpFixture({ beforeKey: () => {
        const state = f.history();
        assert.equal(state.history.reservedActions, 57); assert.equal(state.history.sentActions, 55);
        assert.equal(state.pending.status, 'SENT'); assert.equal(state.pending.tool, 'key');
        assert.equal(state.pending.runID, f.runID);
        const attempt = JSON.parse(readFileSync(resolve(f.output, 'call-007.attempt.private.json'), 'utf8'));
        assert.equal(attempt.tool, 'key'); assert.deepEqual(attempt.args, escapeArgs);
        assert.equal(attempt.outerProtocolID, 'synthetic-outer-E1-key'); assert.equal(attempt.innerProtocolID, 'UNKNOWN');
        assert.equal(attempt.sourceSha256, sha(f.source)); assert.equal(attempt.runID, f.runID); assert.equal(attempt.batch, 'E1');
        assert.equal(Object.hasOwn(attempt, 'reservationID'), false, 'Transport submission cannot invent a guard reservation');
        assert.ok(requiredOwnedHttpFiles(f.root, f.provider.options.powershell, /** @type {string} */ (f.provider.options.supervisor))
            .includes(resolve(f.root, 'tools/starsector-proxy/campaign-authority.mjs')));
    } });
    let batch;
    try {
        batch = await startOwnedHttpBatch(f.provider.options, { spawn: f.provider.spawn });
        await f.before(batch);
        const result = await batch.call('key', escapeArgs, { outerProtocolID: 'synthetic-outer-E1-key' });
        assert.notEqual(result.isError, true); assert.deepEqual(f.keys, [['escape', 1]]);
        const call = JSON.parse(readFileSync(resolve(batch.output, 'call-007.private.json'), 'utf8'));
        assert.equal(call.outerProtocolID, 'synthetic-outer-E1-key'); assert.equal(typeof call.innerProtocolID, 'number');
        assert.deepEqual(call.args, escapeArgs); assert.equal(call.tool, 'key');
        assert.equal(f.history().pending.status, 'AWAITING_AFTER'); assert.equal(f.history().uncertain, false);
        assert.equal(f.provider.starts(), 1);
        const terminal = await batch.close();
        assert.equal(terminal.terminalObserved, true); assert.equal(terminal.cleanupUnknown, false);
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.terminalBoundToChild, true);
        assert.equal(directToolNames.includes('key'), false, 'Original frozen V1 raw set remains unchanged');
    } finally { await batch?.close(); await f.close(); }
}));

test('owned HTTP V1 refuses key without a native call', async () => {
    let nativeCalls = 0;
    const f = directFixture({ dispatch: async () => { nativeCalls++; return { content: [] }; } }); let batch;
    try {
        batch = await startOwnedHttpBatch(f.options, { spawn: f.spawn });
        await assert.rejects(batch.call('key', escapeArgs), /unsupported raw call/i);
        assert.equal(nativeCalls, 0);
    } finally { await batch?.close(); await f.close(); }
});

test('owned HTTP V2 E2 refuses key before another reservation or native effect', async () => consenting(async () => {
    const f = await campaignHttpFixture({ e2: true }); let batch;
    try {
        batch = await startOwnedHttpBatch(f.provider.options, { spawn: f.provider.spawn });
        const before = readFileSync(f.historyPath);
        await assert.rejects(batch.call('key', escapeArgs), /unsupported raw call/i);
        assert.ok(readFileSync(f.historyPath).equals(before)); assert.deepEqual(f.keys, []);
    } finally { await batch?.close(); await f.close(); }
}));

test('owned HTTP V2 E1 reserves an attempted non-Escape key before refusal and sends no native key', async () => consenting(async () => {
    const f = await campaignHttpFixture(); let batch;
    try {
        batch = await startOwnedHttpBatch(f.provider.options, { spawn: f.provider.spawn }); await f.before(batch);
        try {
            const result = await batch.call('key', { ...escapeArgs, text: 'space' }); assert.equal(result.isError, true);
        } catch (error) { assert.match(String(error), /Batch input refused: only one exact Escape is qualified/); }
        assert.deepEqual(f.keys, []);
        assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 54);
        assert.equal(f.history().uncertain, true);
    } finally { await batch?.close(); await f.close(); }
}));

test('uncertain HTTP Escape cannot send a repeat or replace the launch input result', async () => consenting(async () => {
    const f = await campaignHttpFixture({ uncertainKey: true }); let batch;
    try {
        batch = await startOwnedHttpBatch(f.provider.options, { spawn: f.provider.spawn }); await f.before(batch);
        const result = await batch.call('key', escapeArgs); assert.equal(result.isError, true);
        assert.deepEqual(f.keys, [['escape', 1]]); assert.equal(f.history().uncertain, true);
        await assert.rejects(batch.call('key', escapeArgs), /closed/i);
        assert.deepEqual(f.keys, [['escape', 1]]);
        assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
    } finally { await batch?.close(); await f.close(); }
}));
