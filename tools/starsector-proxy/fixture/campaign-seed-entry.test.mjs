// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { seedInputSession, seedTarget } from './campaign-input-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';

test('seed entry permits only its three declared clicks then requires inspected paused seed proof before Escape', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    try {
        f = await completedSeedLaunch();
        const runID = 'seeded-entry-three-clicks-pause-required';
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID }));
        /** @type {string[]} */ const effects = [];
        h = seedInputSession({ ...f, runID }, tool => effects.push(tool), {
            window: () => ({ ...seedTarget, windowId: 21 }) });
        const originalClock = f.ledger().providerClock;
        for (const coordinate of [[40, 80], [50, 90], [60, 100]]) {
            await h.before();
            const result = await h.call('left_click', { target_window_id: 21, target_app: 'java.exe',
                focus_strategy: 'strict', coordinate });
            assert.notEqual(result.isError, true, JSON.stringify(result));
        }
        assert.deepEqual(effects, ['left_click', 'left_click', 'left_click']);
        const state = f.ledger();
        assert.equal(state.history.reservedActions, 4); assert.equal(state.history.sentActions, 4);
        assert.equal(state.pending.status, 'AWAITING_AFTER'); assert.equal(state.pending.tool, 'left_click');
        assert.equal(state.resolvedActions.length, 2);
        assert.deepEqual(state.providerClock, originalClock);
        await h.before();
        await assert.rejects(async () => h?.call('key', { text: 'escape', repeat: 1, target_window_id: 21,
            target_app: 'java.exe', focus_strategy: 'strict' }), /paused seed inspection required/);
        assert.deepEqual(effects, ['left_click', 'left_click', 'left_click']);
        assert.equal(f.ledger().uncertain, true);
        assert.equal(f.ledger().history.reservedActions, 4);
    } finally {
        await h?.embedded.server.close(); await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
