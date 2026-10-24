// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { seedInputSession, claimSeededLauncher } from './campaign-input-test-support.mjs';

test('phase-local immutable seeded B1 authority keeps exact finite consent through real vendor Play', async () => {
    const f = campaignLaunchFixture();
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {unknown[][]} */ const effects = [];
    try {
        const authority = claimSeededLauncher(f, f.runID);
        h = seedInputSession(authority, (tool, args) => {
            const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
            assert.equal(state.history.reservedActions, 1);
            assert.equal(state.history.sentActions, 1);
            effects.push([tool, ...args]);
        });
        await h.before();
        const result = await h.call('left_click', { target_window_id: 19,
            target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] });
        assert.notEqual(result.isError, true, JSON.stringify(result));
        assert.deepEqual(effects, [['left_click', 40, 80, 'left', 1]]);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
