// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
import { seedInputSession } from './campaign-input-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';

test('seeded B1 reserves Play in the original nonzero launch history before the real vendor effect', async () => {
    const f = campaignLaunchFixture({ historyBaseline: { overallAttempts: 33, uiAttempts: 31,
        reservedActions: 56, sentActions: 54 } });
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {unknown[][]} */ const effects = [];
    try {
        const campaign = { ...f.campaign, flow: 'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE' };
        const planPath = resolve(f.scenario, 'B1-plan.private.json'), leasePath = resolve(f.scenario, 'B1-lease.private.json');
        json(planPath, { ...f.phasePlan, campaign, batches: [{ id: 'B1' }] });
        json(leasePath, { ...f.phaseLease, schemaVersion: 3, phase: campaign.flow, campaign, planSha256: filePin(planPath).sha256 });
        const ownedReceiptPath = resolve(f.scenario, 'driver-B1-owned.private.json');
        json(ownedReceiptPath, JSON.parse(readFileSync(resolve(f.scenario, 'driver-E1-owned.private.json'), 'utf8')));
        const source = resolve(f.scenario, 'B1-authorization.private.json');
        json(source, { schemaVersion: 3, scenario: f.scenario, batch: 'B1', planPath,
            planSha256: filePin(planPath).sha256, leasePath, leaseSha256: filePin(leasePath).sha256,
            ownedReceiptPath, ownedReceiptSha256: filePin(ownedReceiptPath).sha256 });
        const authority = { ...f, source, runID: 'seeded-b1-global-history' };
        claimBatchAuthorization(readBatchAuthorization(source, { root: f.root, runID: authority.runID }));
        h = seedInputSession(authority, (tool, args) => {
            const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
            assert.deepEqual(state.history, { overallAttempts: 33, uiAttempts: 31, reservedActions: 57, sentActions: 55 });
            assert.equal(state.pending.batch, 'B1');
            assert.equal(state.pending.tool, 'left_click');
            effects.push([tool, ...args]);
        });
        await h.before();
        const result = await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] });
        assert.notEqual(result.isError, true, JSON.stringify(result));
        assert.deepEqual(effects, [['left_click', 40, 80, 'left', 1]]);
        const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
        assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.equal(state.uncertain, false);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
