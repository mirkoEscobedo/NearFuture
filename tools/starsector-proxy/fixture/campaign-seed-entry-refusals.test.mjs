// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { seedInputSession, seedTarget } from './campaign-input-test-support.mjs';
import { claimBatchAuthorization, readBatchAuthorization } from '../batch-authorization.mjs';

for (const fault of ['wrong Load coordinate', 'premature Escape', 'missing original', 'missing fresh focus'])
    test('seed entry refuses first input with ' + fault + ' before native effects', async () => {
        const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
        const old = names.map(name => process.env[name]);
        process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
        process.env.COMPUTER_USE_AUDIT_LOG = 'false';
        /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
        /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
        try {
            f = await completedSeedLaunch();
            const runID = 'seed-entry-first-input-refused';
            claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID }));
            /** @type {string[]} */ const effects = [];
            h = seedInputSession({ ...f, runID }, tool => effects.push(tool), {
                window: () => ({ ...seedTarget, windowId: 21 }) });
            if (fault === 'missing original' || fault === 'missing fresh focus') {
                await h.call('doctor', {}); await h.call('policy_status', {}); await h.call('get_window', { window_id: 21 });
                if (fault === 'missing fresh focus') await h.call('screenshot', { target_window_id: 21, quality: 0, width: 480 });
            } else await h.before();
            const client = h;
            if (fault === 'premature Escape') await assert.rejects(() => client.call('key', {
                text: 'escape', repeat: 1, target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict' }),
            /mutation outside remaining sequence/);
            else await assert.rejects(() => client.call('left_click', { target_window_id: 21,
                target_app: 'java.exe', focus_strategy: 'strict', coordinate: fault === 'wrong Load coordinate' ? [41, 80] : [40, 80] }),
            /inspected Load Game coordinate|original BEFORE receipt|strict fresh focus/);
            assert.deepEqual(effects, []);
            const state = f.ledger();
            assert.equal(state.history.reservedActions, fault === 'premature Escape' ? 1 : 2);
            assert.equal(state.history.sentActions, 1); assert.equal(state.uncertain, true);
        } finally {
            await h?.embedded.server.close(); await f?.close();
            names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
        }
    });
