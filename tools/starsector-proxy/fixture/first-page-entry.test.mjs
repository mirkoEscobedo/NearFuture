// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { seedInputSession } from './campaign-input-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';

test('first-page inspection transfers only known closed Play then sends one inspected New Game click and observes without advancing', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    try {
        f = await completedSeedLaunch({ inspection: true });
        const authority = { ...f, runID: 'first-page-one-new-game' };
        claimBatchAuthorization(readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }));
        h = seedInputSession(authority, tool => effects.push(tool), { window: () => ({
            pid: 101, windowId: 21, title: 'Starsector', bundleId: 'java.exe', isFocused: true, isOnScreen: true,
            bounds: { x: 10, y: 10, width: 480, height: 220 } }) });
        await h.before();
        const args = { target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] };
        assert.notEqual((await h.call('left_click', args)).isError, true);
        await h.before();
        assert.deepEqual(f.ledger().history, { overallAttempts: 3, uiAttempts: 2, reservedActions: 2, sentActions: 2 });
        const active = h;
        await assert.rejects(() => active.call('left_click', args), /remaining sequence/);
        assert.deepEqual(effects, ['left_click']);
        assert.equal(f.ledger().pending.tool, 'left_click');
        assert.equal(f.ledger().pending.status, 'AWAITING_AFTER');
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), f.originalLaunchBytes);
    } finally {
        await h?.embedded.server.close(); await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
