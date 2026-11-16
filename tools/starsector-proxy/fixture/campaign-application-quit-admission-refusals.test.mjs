// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';
import { json } from './seed-test-support.mjs';

/** @type {Array<[string,(f:ReturnType<typeof campaignLaunchFixture>)=>void]>} */
const faults = [
    ['wrong root control version', f => { f.plan.campaignControl.schemaVersion = 5; }],
    ['unknown root profile', f => { f.plan.campaignControl.profile += '_SAVE'; }],
    ['extra root authority', f => { Object.assign(f.plan.campaignControl, { saveAllowed: true }); }],
    ['unpaired immutable control', f => { f.launchLease.campaignControl = { schemaVersion: 1, profile: 'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE' }; }],
    ['ninth action ceiling', f => { f.plan.bounds.maxScenarioActionsIncludingActivations = 9; f.launchLease.maxActions = 9; }],
    ['fourth provider', f => { f.plan.bounds.maxProxyInvocations = f.launchLease.maxProxyInvocations = 4; }],
    ...['seed-application-quit-profile.mjs', 'seed-scene-inspection.mjs'].map(name => /** @type {[string,(f:ReturnType<typeof campaignLaunchFixture>)=>void]} */ ([
        'missing current ' + name, f => {
            const path = resolve(f.scenario, 'frozen-support-pins.private.json');
            const pins = JSON.parse(readFileSync(path, 'utf8'));
            pins.pins = pins.pins.filter((/** @type {any} */ pin) => !pin.path.endsWith('/' + name) && !pin.path.endsWith('\\' + name));
            const pin = json(path, pins);
            f.launchLease.supportPinsSha256 = f.phaseLease.supportPinsSha256 = pin.sha256;
        }]))
];

for (const [name, change] of faults) test('both real immutable launch validators refuse ' + name, () => {
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } });
    try {
        change(f); f.refreshLaunch();
        const ps = f.admit();
        assert.notEqual(ps.status, 0, name + ' must refuse before launch');
        const authority = claimSeededLauncher(f, 'synthetic-quit-closed-launch', false);
        assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }));
    } finally { f.close(); }
});
