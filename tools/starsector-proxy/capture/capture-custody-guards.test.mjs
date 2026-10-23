// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync, readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
const refusal = { code: -32603, message: 'NATIVE99_CAPTURE_CUSTODY_REFUSED' };
const input = { name: 'left_click', arguments: { coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } };

test('missing capture intent refuses before native dispatch and latches all later requests', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const h = await openCaptureClient(f, {
        readIntent: () => { throw new Error('ENOENT E:/private/not-for-wire/intent.json'); },
        dispatch: async name => { calls.push(name); return expectedImage; },
    });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        await assert.rejects(h.client.callTool(input), refusal);
        await assert.rejects(h.client.callTool({ name: 'wait', arguments: { duration: 0.1 } }), refusal);
        assert.deepEqual(calls, []);
        assert.equal(existsSync(f.slot.callbackReceipt), false);
        const failure = JSON.parse(readFileSync(resolve(f.output, 'raw-capture-custody-failure.private.json'), 'utf8'));
        assert.equal(failure.status, 'HOLD');
        assert.equal(failure.noFurtherNativeDispatch, true);
        assert.match(failure.error.message, /not-for-wire/);
    } finally { await h.close(); }
});

test('stale, mismatched and unowned caller data cannot dispatch a capture or later input', { timeout: 10000 }, async t => {
    const cases = [
        ['expired batch', (/** @type {any} */ x) => { x.batchClock.deadlineMs = 999999; }],
        ['wrong source pins', (/** @type {any} */ x) => { x.sourcePins[0].sha256 = '0'.repeat(64); }],
        ['unknown frame', (/** @type {any} */ x) => { x.frameId = 'F9'; }],
        ['wrong target', (/** @type {any} */ x) => { x.args = { ...x.args, target_window_id: 22 }; }],
        ['unowned process', (/** @type {any} */ x) => { x.ownedWitness.row.inSupervisorJob = false; }],
        ['wrong root record', (/** @type {any} */ x) => { x.rootCallRecord.key = 'other-root'; }],
    ];
    for (const [name, mutate] of cases) await t.test(/** @type {string} */ (name), async () => {
        const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
        /** @type {(intent: any) => void} */ (mutate)(f.intent);
        writeFileSync(f.intentPath, JSON.stringify(f.intent) + '\n');
        const h = await openCaptureClient(f, { dispatch: async name => { calls.push(name); return expectedImage; } });
        try {
            await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
            await assert.rejects(h.client.callTool(input), refusal);
            assert.deepEqual(calls, []);
            assert.equal(existsSync(f.slot.callbackReceipt), false);
            assert.equal(existsSync(f.slot.png), false);
        } finally { await h.close(); }
    });
});
