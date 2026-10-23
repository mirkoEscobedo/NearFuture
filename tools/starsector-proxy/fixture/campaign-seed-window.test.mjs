// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedInputSession, seedTarget } from './campaign-input-test-support.mjs';

test('seeded B1 observes a replacement game HWND on the same owned PID only after known Play', async () => {
    const f = campaignLaunchFixture();
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    try {
        const authority = claimSeededLauncher(f, 'seeded-b1-known-window-handoff');
        let window = { ...seedTarget };
        /** @type {string[]} */ const effects = [];
        h = seedInputSession(authority, tool => effects.push(tool), { window: () => window });
        await h.before();
        const play = await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] });
        assert.notEqual(play.isError, true);
        window = { ...seedTarget, windowId: 21 };
        await h.call('get_window', { window_id: 21 });
        const after = await h.call('screenshot', { target_window_id: 21, quality: 0, width: 480 });
        assert.equal(after.content.filter((/** @type {any} */ item) => item.type === 'image').length, 1);
        assert.deepEqual(effects, ['left_click']);
        await assert.rejects(async () => h?.call('left_click', { target_window_id: 21, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] }), /mutation outside remaining sequence/);
        assert.deepEqual(effects, ['left_click']);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
