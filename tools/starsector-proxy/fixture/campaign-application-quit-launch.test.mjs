// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';

test('the explicit no-save return-menu Quit profile admits the same immutable launch through PowerShell and JavaScript', () => {
    const f = campaignLaunchFixture({ campaignControl: {
        schemaVersion: 6, profile: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' } });
    try {
        const ps = f.admit();
        const authority = claimSeededLauncher(f, 'synthetic-application-quit-admission', false);
        /** @type {unknown} */ let js;
        try {
            const admitted = readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID });
            const binding = admitted.binding;
            assert.ok('campaign' in binding);
            js = { schemaVersion: binding.schemaVersion, flow: binding.campaign.flow };
        } catch (error) {
            js = { error: error instanceof Error ? error.message : String(error) };
        }
        assert.deepEqual({ powershell: ps.status, javascript: js }, {
            powershell: 0, javascript: {
                schemaVersion: 3, flow: 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' }
        }, ps.stderr);
    } finally { f.close(); }
});
