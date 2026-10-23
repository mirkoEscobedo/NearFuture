// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { custodyFixture, expectedImage, screenshotArgs, png } from './capture-custody-fixture.mjs';
const refusal = { code: -32603, message: 'NATIVE99_CAPTURE_CUSTODY_REFUSED' };
test('a parallel metadata read stays available but a second capture latches without another native capture', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    let start = () => {}, release = () => {};
    const started = new Promise(done => { start = () => done(undefined); });
    const gate = new Promise(done => { release = () => done(undefined); });
    const readResult = { content: [{ type: 'text', text: 'SYNTHETIC_READ_ONLY' }] };
    const h = await openCaptureClient(f, { dispatch: async name => {
        calls.push(name);
        if (name !== 'screenshot') return readResult;
        start(); await gate; return expectedImage;
    } });
    /** @type {Promise<any>|undefined} */ let first;
    try {
        first = h.client.callTool({ name: 'screenshot', arguments: { ...screenshotArgs } });
        void first.catch(() => {});
        await started;
        assert.deepEqual(await h.client.callTool({ name: 'wait', arguments: { duration: 0.1 } }), readResult);
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: { ...screenshotArgs } }), refusal);
        release();
        await assert.rejects(first, refusal);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'capture-001.latched.raw.private.json'), 'utf8')), expectedImage);
        assert.equal(existsSync(f.slot.callbackReceipt), false);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot', 'wait']);
        assert.equal(f.reads(), 1);
    } finally { release(); await first?.catch(() => {}); await h.close(); }
});
test('a consumed capture intent cannot dispatch twice or replace its first original proof', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const h = await openCaptureClient(f, { dispatch: async name => { calls.push(name); return expectedImage; } });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: { ...screenshotArgs } }), expectedImage);
        const proofBytes = readFileSync(f.slot.callbackReceipt);
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: { ...screenshotArgs } }), refusal);
        assert.deepEqual(readFileSync(f.slot.callbackReceipt), proofBytes);
        assert.deepEqual(readFileSync(f.slot.png), png);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});
