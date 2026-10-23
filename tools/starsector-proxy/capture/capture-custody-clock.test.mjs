// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync, readFileSync, existsSync } from 'node:fs';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
import { openCaptureClient } from './capture-custody-test-client.mjs';
const refusal = { code: -32603, message: 'NATIVE99_CAPTURE_CUSTODY_REFUSED' };
test('the existing exact 90-second capture and 60-second persistence boundaries remain usable', { timeout: 10000 }, async () => {
    const f = custodyFixture();
    let time = 1040000;
    const h = await openCaptureClient(f, { now: () => time, dispatch: async () => { time = 1070000; return expectedImage; } });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), expectedImage);
        const proof = JSON.parse(readFileSync(f.slot.callbackReceipt, 'utf8'));
        assert.equal(proof.startMs, 1040000); assert.equal(proof.endMs, 1070000);
    } finally { await h.close(); }
});
test('capture below 90 seconds and altered root or batch bounds refuse before native effects', { timeout: 10000 }, async t => {
    /** @type {Array<[string, (intent: any) => void, number]>} */
    const cases = [
        ['89999 milliseconds remain', () => {}, 1040001],
        ['root deadline extended', x => { x.rootClock.deadlineMs++; }, 1000000],
        ['caller deadline extended', x => { x.rootClock.callerAdmissionDeadlineMs++; }, 1000000],
        ['batch deadline extended', x => { x.batchClock.deadlineMs++; }, 1000000],
    ];
    for (const [name, mutate, time] of cases) await t.test(name, async () => {
        const f = custodyFixture(); let dispatches = 0;
        mutate(f.intent); writeFileSync(f.intentPath, JSON.stringify(f.intent) + '\n');
        const h = await openCaptureClient(f, { now: () => time, dispatch: async () => { dispatches++; return expectedImage; } });
        try {
            await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
            assert.equal(dispatches, 0);
            assert.equal(existsSync(f.slot.callbackReceipt), false);
        } finally { await h.close(); }
    });
});
test('a clock regression during native dispatch cannot produce a positive completion receipt', { timeout: 10000 }, async () => {
    const f = custodyFixture();
    let time = 1000000;
    const h = await openCaptureClient(f, { now: () => time, dispatch: async () => { time = 999999; return expectedImage; } });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        assert.equal(existsSync(f.slot.callbackReceipt), false);
    } finally { await h.close(); }
});

test('a clock regression after file persistence cannot commit the final completion', { timeout: 10000 }, async () => {
    const f = custodyFixture(), samples = [1000000, 1000000, 999999];
    const h = await openCaptureClient(f, { now: () => samples.shift() ?? 999999, dispatch: async () => expectedImage });
    try {
        await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), refusal);
        assert.equal(existsSync(f.slot.callbackReceipt), false);
        assert.equal(existsSync(f.slot.png), true, 'partial originals remain evidence, never a completion award');
    } finally { await h.close(); }
});
