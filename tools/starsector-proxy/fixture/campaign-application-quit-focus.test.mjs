// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedInputSession, seedTarget } from './campaign-input-test-support.mjs';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { readClaimedBatchAuthorization } from '../batch-authorization.mjs';
import { reserveCampaignAction } from '../campaign-authority.mjs';

test('the eight-action application Quit route refuses initial activation before reservation or a native effect', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } });
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    try {
        const authority = claimSeededLauncher(f, 'synthetic-quit-no-activation');
        h = seedInputSession(authority, tool => effects.push(tool));
        await h.before();
        await assert.rejects(h.call('activate_window', { window_id: 19, timeout_ms: 2000 }),
            /mutation outside remaining sequence|application Quit refuses activation|Unknown tool|not registered/);
        assert.deepEqual(effects, []);
        const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
        assert.equal(state.history.reservedActions, 0);
        assert.equal(state.history.sentActions, 0);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const applicationQuit of [true, false]) test(applicationQuit ?
    'naturally focused application Quit Play is accepted and direct activation cannot change the ledger' :
    'legacy seeded launcher still permits its optional activation and Play', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof campaignLaunchFixture> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    try {
        f = campaignLaunchFixture(applicationQuit ? { campaignControl: {
            schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } } : {});
        const authority = claimSeededLauncher(f, applicationQuit ? 'synthetic-quit-focused' : 'synthetic-legacy-activation');
        h = seedInputSession(authority, tool => effects.push(tool));
        await h.before();
        if (applicationQuit) {
            const ledger = resolve(f.scenario, 'guard-launch-history.private.json');
            const before = readFileSync(ledger);
            const { binding } = readClaimedBatchAuthorization(authority.source, { root: f.root, runID: authority.runID });
            assert.throws(() => reserveCampaignAction(binding, { window_id: 19, timeout_ms: 2000 }, 'activate_window'),
                /application Quit refuses activation/);
            assert.deepEqual(readFileSync(ledger), before);
        } else {
            assert.notEqual((await h.call('activate_window', { window_id: 19, timeout_ms: 2000 })).isError, true);
            await h.before();
        }
        assert.notEqual((await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [100, 100] })).isError, true);
        assert.deepEqual(effects, applicationQuit ? ['left_click'] : ['activate_window', 'left_click']);
    } finally {
        await h?.embedded.server.close(); f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

test('a restored focus observation cannot reuse an unfocused original for application Quit Play', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } });
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    const window = { ...seedTarget, isFocused: false };
    try {
        h = seedInputSession(claimSeededLauncher(f, 'synthetic-quit-unfocused-original'),
            tool => effects.push(tool), { window: () => window });
        await h.before();
        window.isFocused = true;
        await h.call('get_frontmost_app', {});
        await h.call('get_window', { window_id: 19 });
        await assert.rejects(h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [100, 100] }), /focused original/);
        assert.deepEqual(effects, []);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
