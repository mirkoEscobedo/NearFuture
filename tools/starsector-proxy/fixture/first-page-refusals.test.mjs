// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { json, write } from './seed-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';

/** @type {Array<[string, (f: any) => void]>} */
const cases = [
    ['unknown profile', f => { f.plan.campaignControl.profile = f.launchLease.campaignControl.profile = 'GENERATE_AND_SAVE'; }],
    ['legacy profile version', f => { f.plan.campaignControl.schemaVersion = f.launchLease.campaignControl.schemaVersion = 1; }],
    ['wizard extension', f => { f.plan.campaignControl.wizardAdvanceAllowed = f.launchLease.campaignControl.wizardAdvanceAllowed = true; }],
    ['E2 declaration', f => { f.plan.batches.push({ id: 'E2' }); }],
    ['copied seed authority', f => { f.plan.seed = f.launchLease.seed = {}; }],
    ['seed manifest', f => { json(f.manifestPath, {}); }],
    ['seed stage authority', f => { json(resolve(f.scenario, 'seed-stage-authority.private.json'), {}); }],
    ['seed stage plan', f => { json(resolve(f.scenario, 'seed-stage-plan.private.json'), {}); }],
    ['seed stage claim', f => { json(resolve(f.scenario, 'seed-stage-claim.private.json'), {}); }],
    ['seed stage receipt', f => { json(resolve(f.scenario, 'seed-stage-receipt.private.json'), {}); }],
    ['private save file', f => { write(resolve(f.installation, 'saves/unexpected.xml'), '<generated/>'); }],
    ['additional action', f => { f.plan.bounds.maxScenarioActionsIncludingActivations = f.launchLease.maxActions = 9; }],
    ['general save permission', f => { f.launchLease.campaignLoadSaveAllowed = true; }],
];
for (const [name, change] of cases) test('first-page ' + name + ' refuses real launch and finite input before effects', () => {
    const f = campaignLaunchFixture({ inspection: true });
    try {
        change(f); f.refreshLaunch();
        const launch = f.admit();
        assert.notEqual(launch.status, 0, name);
        const authority = claimSeededLauncher(f, 'first-page-refusal', false);
        assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }));
    } finally { f.close(); }
});

test('first-page phase cannot change its profile to the older copied-seed grant', () => {
    const f = campaignLaunchFixture({ inspection: true });
    try {
        const authority = claimSeededLauncher(f, 'first-page-no-profile-swap', false);
        const source = JSON.parse(readFileSync(authority.source, 'utf8'));
        json(authority.source, { ...source, schemaVersion: 3 });
        assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }), /matching campaign flow/);
    } finally { f.close(); }
});
