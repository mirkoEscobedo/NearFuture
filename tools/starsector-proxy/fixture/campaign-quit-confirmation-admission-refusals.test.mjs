// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';
import { json } from './seed-test-support.mjs';

/** @type {Array<[string,(f:ReturnType<typeof campaignLaunchFixture>)=>void]>} */
const faults = [
    ['old root version', f => { f.plan.campaignControl.schemaVersion = 6; }],
    ['unknown profile', f => { f.plan.campaignControl.profile += '_SAVE'; }],
    ['unpaired root grant', f => { f.launchLease.campaignControl = {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' }; }],
    ['eight-input ceiling', f => { f.plan.bounds.maxScenarioActionsIncludingActivations = f.launchLease.maxActions = 8; }],
    ['ten-input ceiling', f => { f.plan.bounds.maxScenarioActionsIncludingActivations = f.launchLease.maxActions = 10; }],
    ['fourth provider', f => { f.plan.bounds.maxProxyInvocations = f.launchLease.maxProxyInvocations = 4; }],
    ['longer provider clock', f => { f.plan.bounds.proxyTimeoutMs = 480000; }],
];
for (const [name, change] of faults) test('both real confirmation-root validators refuse ' + name, () => {
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 7, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE' }, maxScenarioActions: 9 });
    try {
        change(f); f.refreshLaunch();
        assert.notEqual(f.admit().status, 0);
        const a = claimSeededLauncher(f, 'synthetic-quit-confirmation-closed', false);
        assert.throws(() => readBatchAuthorization(a.source, { root: f.root, runID: a.runID }));
    } finally { f.close(); }
});

for (const confirmationRoot of [false, true]) test('self-consistent phases cannot replace the immutable ' +
    (confirmationRoot ? 'nine' : 'eight') + '-input root', () => {
    const f = campaignLaunchFixture({ campaignControl: { schemaVersion: confirmationRoot ? 7 : 6,
        profile: confirmationRoot ? 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE' :
            'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' }, maxScenarioActions: confirmationRoot ? 9 : 8 });
    try {
        assert.equal(f.admit().status, 0);
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const a = claimSeededLauncher(f, 'synthetic-quit-confirmation-substitution', false);
        const auth = JSON.parse(readFileSync(a.source, 'utf8'));
        const plan = JSON.parse(readFileSync(auth.planPath, 'utf8'));
        const lease = JSON.parse(readFileSync(auth.leasePath, 'utf8'));
        plan.campaign.flow = lease.campaign.flow = lease.phase = confirmationRoot ?
            'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' : 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE';
        plan.bounds.maxScenarioActionsIncludingActivations = lease.maxActions = confirmationRoot ? 8 : 9;
        lease.planSha256 = auth.planSha256 = json(auth.planPath, plan).sha256;
        auth.leaseSha256 = json(auth.leasePath, lease).sha256; json(a.source, auth);
        assert.throws(() => readBatchAuthorization(a.source, { root: f.root, runID: a.runID }));
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
    } finally { f.close(); }
});

test('confirmation root cannot borrow the generation fixture or phaseauth7 creation selector', () => {
    const f = campaignLaunchFixture({ generation: true, campaignControl: {
        schemaVersion: 7, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE' }, maxScenarioActions: 9 });
    try {
        assert.notEqual(f.admit().status, 0);
        const a = claimSeededLauncher(f, 'synthetic-quit-not-creation', false);
        const auth = JSON.parse(readFileSync(a.source, 'utf8')); auth.schemaVersion = 7; json(a.source, auth);
        assert.throws(() => readBatchAuthorization(a.source, { root: f.root, runID: a.runID }));
    } finally { f.close(); }
});
