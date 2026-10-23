// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedInputSession } from './campaign-input-test-support.mjs';

test('seeded B1 activation then freshly observed Play retains both attempts and the original provider clock', async () => {
    const f = campaignLaunchFixture({ historyBaseline: { overallAttempts: 33, uiAttempts: 31,
        reservedActions: 56, sentActions: 54 } });
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    try {
        const authority = claimSeededLauncher(f, 'seeded-b1-observed-progression');
        const ledger = () => JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
        const originalClock = ledger().providerClock;
        /** @type {string[]} */ const effects = [];
        h = seedInputSession(authority, tool => {
            effects.push(tool);
            if (tool === 'left_click') {
                const state = ledger();
                assert.deepEqual(state.history, { overallAttempts: 33, uiAttempts: 31, reservedActions: 58, sentActions: 56 });
                assert.deepEqual(state.providerClock, originalClock);
                assert.equal(state.resolvedActions.length, 1);
                assert.equal(state.resolvedActions[0].tool, 'activate_window');
                assert.equal(state.resolvedActions[0].after.captureReceipt, true);
                assert.equal(state.resolvedActions[0].after.freshness, 'UNKNOWN');
                assert.match(state.resolvedActions[0].after.pngSha256, /^[a-f0-9]{64}$/);
            }
        });
        await h.before();
        const activated = await h.call('activate_window', { window_id: 19, timeout_ms: 2000 });
        assert.notEqual(activated.isError, true, JSON.stringify(activated));
        await h.before();
        const play = await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] });
        assert.notEqual(play.isError, true, JSON.stringify(play));
        assert.deepEqual(effects, ['activate_window', 'left_click']);
        assert.equal(ledger().pending.tool, 'left_click');
        assert.equal(ledger().pending.status, 'AWAITING_AFTER');
        assert.equal(ledger().uncertain, false);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const failure of ['missing AFTER', 'truncated AFTER']) {
    test('seeded B1 preserves activation and refuses Play with ' + failure, async () => {
        const f = campaignLaunchFixture();
        const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
        const old = names.map(name => process.env[name]);
        process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
        process.env.COMPUTER_USE_AUDIT_LOG = 'false';
        /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
        try {
            const authority = claimSeededLauncher(f, 'seeded-b1-refuse-' + failure.split(' ')[0]);
            /** @type {string[]} */ const effects = [];
            let corrupt = false;
            h = seedInputSession(authority, tool => effects.push(tool), { screenshot: () => corrupt ? {
                content: [{ type: 'image', mimeType: 'image/png', data: 'iVBORw0KGgo=' }] } : undefined });
            await h.before();
            await h.call('activate_window', { window_id: 19, timeout_ms: 2000 });
            if (failure === 'truncated AFTER') {
                corrupt = true;
                await assert.rejects(async () => h?.before(), /Complete original PNG/);
            } else {
                await h.call('get_frontmost_app', {});
                await h.call('get_window', { window_id: 19 });
                await assert.rejects(() => h?.call('left_click', { target_window_id: 19, target_app: 'java.exe',
                    focus_strategy: 'strict', coordinate: [40, 80] }), /known original AFTER/);
            }
            assert.deepEqual(effects, ['activate_window']);
            const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
            assert.deepEqual(state.history, { overallAttempts: 3, uiAttempts: 2, reservedActions: 1, sentActions: 1 });
            assert.equal(state.pending.tool, 'activate_window');
            assert.equal(state.pending.status, 'AWAITING_AFTER');
            assert.equal(state.uncertain, true);
            assert.equal(state.resolvedActions, undefined);
        } finally {
            await h?.embedded.server.close(); f.close();
            names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
        }
    });
}
