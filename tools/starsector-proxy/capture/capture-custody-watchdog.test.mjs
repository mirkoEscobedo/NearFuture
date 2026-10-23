// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync, existsSync, readFileSync } from 'node:fs';
import { custodyFixture, expectedImage, screenshotArgs } from './capture-custody-fixture.mjs';
import { openCaptureClient } from './capture-custody-test-client.mjs';
import { createCaptureCustody } from './capture-custody.mjs';

/** Synthetic caller/producer agreement, never a native session receipt. */
function guidedFixture() {
    const f = /** @type {any} */ (custodyFixture('guided-watchdog-'));
    f.admitted.rootSessionWatchdogMs = 7200000;
    f.admitted.rootRunnerCommandExact += ' -SessionWatchdogMs 7200000';
    f.intent.rootCallRecord.value.args.cmd = f.admitted.rootRunnerCommandExact;
    const root = f.intent.rootClock;
    root.sessionWatchdogMs = 7200000;
    root.deadlineMs = root.startMs + 7200000;
    root.callerAdmissionDeadlineMs = root.deadlineMs;
    root.originalCallerAdmissionDeadlineMs = root.deadlineMs;
    root.effectiveDeadlineMs = root.deadlineMs;
    Object.assign(f.intent.producerWitness, {
        sessionWatchdogMs: 7200000,
        conservativeOuterDeadlineUtc: '1970-01-01T02:15:00.0000000Z',
        controllerDeadlineUtc: '1970-01-01T02:15:00.0000000Z',
        effectiveDeadlineMs: root.deadlineMs,
    });
    // A new individual capture remains 180s, even after the legacy session budget.
    f.intent.batchClock = { startMs: 1950000, lastMs: 1999900, deadlineMs: 2130000 };
    f.intent.createdMs = 1999950;
    f.intent.ownedWitness.atMs = 1999900;
    f.intent.ownedWitness.row.mainWindowObservedAtUtc = new Date(1999900).toISOString();
    f.now = () => 2000000;
    return f;
}
test('an explicitly admitted two-hour watchdog permits a fresh capture after the legacy session budget', async () => {
    const f = guidedFixture();
    writeFileSync(f.intentPath, JSON.stringify(f.intent) + '\n');
    let calls = 0;
    const h = await openCaptureClient(f, { dispatch: async () => { calls++; return expectedImage; } });
    try {
        assert.deepEqual(await h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }), expectedImage);
        assert.equal(calls, 1);
        const receipt = JSON.parse(readFileSync(f.slot.callbackReceipt, 'utf8'));
        assert.equal(receipt.callerRootClock.sessionWatchdogMs, 7200000);
        assert.equal(receipt.callerBatchClock.deadlineMs - receipt.callerBatchClock.startMs, 180000);
    } finally { await h.close(); }
});
test('admitted watchdog must be an explicit bounded integer when present', () => {
    for (const value of [null, true, '7200000', 479999, 43200001, 7200000.5]) {
        const f = guidedFixture(); f.admitted.rootSessionWatchdogMs = value;
        assert.throws(() => createCaptureCustody({ admitted: f.admitted, readIntent: f.readIntent, now: f.now }),
            { message: 'ADMITTED_SESSION_WATCHDOG_REQUIRED' });
    }
});
test('unbound or mismatched guided watchdog data refuses before dispatch', async t => {
    /** @type {Array<[string, (f: any) => void]>} */
    const cases = [
        ['no admitted extension', f => { delete f.admitted.rootSessionWatchdogMs; }],
        ['root disagrees', f => { f.intent.rootClock.sessionWatchdogMs = 480000; }],
        ['producer disagrees', f => { f.intent.producerWitness.sessionWatchdogMs = 480000; }],
        ['root deadline reset', f => { f.intent.rootClock.deadlineMs++; }],
    ];
    for (const [name, mutate] of cases) await t.test(name, async () => {
        const f = guidedFixture(); mutate(f);
        writeFileSync(f.intentPath, JSON.stringify(f.intent) + '\n');
        let calls = 0;
        const h = await openCaptureClient(f, { dispatch: async () => { calls++; return expectedImage; } });
        try {
            await assert.rejects(h.client.callTool({ name: 'screenshot', arguments: screenshotArgs }),
                { code: -32603, message: 'NATIVE99_CAPTURE_CUSTODY_REFUSED' });
            assert.equal(calls, 0);
            assert.equal(existsSync(f.slot.callbackReceipt), false);
        } finally { await h.close(); }
    });
});
