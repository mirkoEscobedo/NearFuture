// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';
import { withApplicationQuit } from './campaign-application-quit-test-support.mjs';
import { json } from './seed-test-support.mjs';

/** @param {string} path */
const read = path => JSON.parse(readFileSync(path, 'utf8'));

for (const newRoot of [false, true]) test('a self-consistent phase cannot switch the immutable ' + (newRoot ? 'Quit' : 'legacy') + ' launch profile', () => {
    const f = campaignLaunchFixture(newRoot ? { campaignControl: {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } } : {});
    try {
        const authority = claimSeededLauncher(f, 'synthetic-quit-phase-substitution', false);
        const auth = read(authority.source), plan = read(auth.planPath), lease = read(auth.leasePath);
        plan.campaign.flow = lease.campaign.flow = lease.phase = newRoot ?
            'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE' : 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE';
        lease.planSha256 = json(auth.planPath, plan).sha256;
        auth.planSha256 = lease.planSha256; auth.leaseSha256 = json(auth.leasePath, lease).sha256;
        json(authority.source, auth);
        assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }),
            /closed seed entry launch profile/);
    } finally { f.close(); }
});

test('Quit E2 refuses a matching legacy Exit game Yes declaration before another action', () => withApplicationQuit(async f => {
    const auth = read(f.source), plan = read(auth.planPath), lease = read(auth.leasePath);
    plan.exitConfirmation = lease.exitConfirmation = { schemaVersion: 1, dialogText: 'Exit game?', confirmLabel: 'Yes' };
    lease.planSha256 = json(auth.planPath, plan).sha256;
    auth.planSha256 = lease.planSha256; auth.leaseSha256 = json(auth.leasePath, lease).sha256;
    json(f.source, auth);
    const before = f.ledger();
    assert.throws(() => readBatchAuthorization(f.source, { root: f.root, runID: f.runID }),
        /closed matching exit confirmation declaration/);
    assert.deepEqual(f.effects, []); assert.deepEqual(f.ledger(), before);
}));
