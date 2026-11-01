// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';

test('both real validators admit the separately paired nine-input Quit confirmation root', () => {
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 7, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE' }, maxScenarioActions: 9 });
    try {
        const ps = f.admit();
        const authority = claimSeededLauncher(f, 'synthetic-quit-confirmation-admission', false);
        let js;
        try {
            const { binding } = readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID });
            assert.ok('campaign' in binding);
            js = { schemaVersion: binding.schemaVersion, flow: binding.campaign.flow,
                maxReservedActions: binding.campaign.maxReservedActions };
        } catch (error) { js = { error: error instanceof Error ? error.message : String(error) }; }
        assert.deepEqual({ powershell: ps.status, javascript: js }, {
            powershell: 0, javascript: { schemaVersion: 3,
                flow: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE', maxReservedActions: 9 }
        }, ps.stderr);
    } finally { f.close(); }
});
