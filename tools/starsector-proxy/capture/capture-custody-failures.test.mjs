// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, existsSync, mkdirSync, lstatSync } from 'node:fs';
import { resolve } from 'node:path';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { custodyFixture, expectedImage, screenshotArgs, png } from './capture-custody-fixture.mjs';
const refusal = { code: -32603, message: 'NATIVE99_CAPTURE_CUSTODY_REFUSED' };
test('an argument association failure retains the actual original result and blocks later input', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const h = await openCaptureClient(f, { dispatch: async (name, args) => {
        calls.push(name);
        args.target_window_id = 22; // Deliberately hostile synthetic native boundary.
        return expectedImage;
    } });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: { ...screenshotArgs } }), refusal);
        const rejectedRaw = resolve(f.output, 'capture-001.rejected-result.raw.private.json');
        assert.ok(existsSync(rejectedRaw), 'an actual returned image must survive failed association checks');
        assert.deepEqual(JSON.parse(readFileSync(rejectedRaw, 'utf8')), expectedImage);
        assert.equal(existsSync(f.slot.callbackReceipt), false);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});

test('native capture errors retain their original wire result and latch later input', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const nativeError = { content: [{ type: 'text', text: 'SYNTHETIC_NATIVE_CAPTURE_REFUSAL' }], isError: true };
    const h = await openCaptureClient(f, { dispatch: async name => { calls.push(name); return nativeError; } });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), nativeError);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'capture-001.raw.private.json'), 'utf8')), nativeError);
        assert.equal(JSON.parse(readFileSync(resolve(f.output, 'capture-001.metadata.private.json'), 'utf8')).captureReceipt, false);
        assert.equal(existsSync(f.slot.png), false); assert.equal(existsSync(f.slot.callbackReceipt), false);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});
test('a failed final receipt retains partial originals without awarding completion or later input', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const h = await openCaptureClient(f, { dispatch: async name => {
        calls.push(name); mkdirSync(f.slot.callbackReceipt); return expectedImage;
    } });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        assert.deepEqual(readFileSync(f.slot.png), png);
        assert.equal(readFileSync(f.slot.payload, 'utf8'), png.toString('base64') + '\n');
        assert.equal(lstatSync(f.slot.callbackReceipt).isFile(), false);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'capture-001.raw.private.json'), 'utf8')), expectedImage);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});
test('an elapsed persistence gate retains the returned raw result and refuses positive capture proof', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    let time = 1000000;
    const h = await openCaptureClient(f, { now: () => time, dispatch: async name => {
        calls.push(name); time = 1070001; return expectedImage;
    } });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'capture-001.late.raw.private.json'), 'utf8')), expectedImage);
        assert.equal(existsSync(f.slot.png), false); assert.equal(existsSync(f.slot.callbackReceipt), false);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});
test('invalid returned PNG retains full raw evidence and latches before another action', { timeout: 10000 }, async () => {
    const f = custodyFixture(), calls = /** @type {string[]} */ ([]);
    const invalid = { content: [{ type: 'image', mimeType: 'image/png', data: png.subarray(0, 24).toString('base64') }] };
    const h = await openCaptureClient(f, { dispatch: async name => { calls.push(name); return invalid; } });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        assert.deepEqual(JSON.parse(readFileSync(resolve(f.output, 'capture-001.raw.private.json'), 'utf8')), invalid);
        assert.equal(existsSync(f.slot.png), false); assert.equal(existsSync(f.slot.callbackReceipt), false);
        await assert.rejects(h.client.callTool({ name: 'left_click', arguments: {
            coordinate: [1, 1], target_window_id: 19, focus_strategy: 'strict' } }), refusal);
        assert.deepEqual(calls, ['screenshot']);
    } finally { await h.close(); }
});
