// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, writeFileSync, unlinkSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedInputSession } from './campaign-input-test-support.mjs';
import { json, filePin, powershell } from './seed-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';

test('first-page inspection admits its real protected no-seed launch and guarded Play without changing original authority', async () => {
    const f = campaignLaunchFixture({ inspection: true });
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    try {
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const launch = f.admit();
        assert.equal(launch.status, 0, launch.stderr);
        const authority = claimSeededLauncher(f, 'first-page-protected-launch', false);
        const source = JSON.parse(readFileSync(authority.source, 'utf8'));
        const campaign = { ...f.campaign, flow: 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE' };
        json(source.planPath, { ...f.phasePlan, campaign, batches: [{ id: 'B1' }] });
        json(source.leasePath, { ...f.phaseLease, schemaVersion: 4, phase: campaign.flow, campaign,
            planSha256: filePin(source.planPath).sha256 });
        json(authority.source, { ...source, schemaVersion: 4, planSha256: filePin(source.planPath).sha256,
            leaseSha256: filePin(source.leasePath).sha256 });
        const admitted = readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID });
        claimBatchAuthorization(admitted);
        h = seedInputSession(authority, tool => effects.push(tool));
        await h.before();
        const result = await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] });
        assert.notEqual(result.isError, true, JSON.stringify(result));
        assert.deepEqual(effects, ['left_click']);
        const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
        assert.deepEqual(state.history, { overallAttempts: 3, uiAttempts: 2, reservedActions: 1, sentActions: 1 });
        assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
    } finally {
        await h?.embedded.server.close(); f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

test('first-page profile reaches both public launch gates over the original no-seed pair before deliberate pre-effect stops', () => {
    const f = campaignLaunchFixture({ inspection: true });
    try {
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const controller = readFileSync(f.controllerPath); unlinkSync(f.controllerPath);
        assert.equal(existsSync(resolve(f.installation, 'jre/bin/java.exe')), false);
        writeFileSync(resolve(f.scenario, 'launch-supervisor.private.json'), 'Controlled pre-effect sentinel');
        try {
            let result = powershell(['-File', resolve(f.root, 'tools/starsector-proxy/fixture/run-native.ps1'),
                '-BindingPath', f.bindingPath, '-BindingSha256', f.bindingPin.sha256, '-EvidenceRoot', f.scenario, '-Phase', 'launch']);
            assert.notEqual(result.status, 0); assert.match(result.stderr, /Preserve existing native evidence/);
            assert.equal(existsSync(resolve(f.scenario, 'supervision-launch')), false);
            const deadline = new Date(Date.now() + 470000).toISOString().replace(/Z$/, '0000Z');
            result = powershell(['-File', resolve(f.root, 'tools/starsector-proxy/fixture/native-controller.ps1'),
                '-BindingPath', f.bindingPath, '-BindingSha256', f.bindingPin.sha256, '-EvidenceRoot', f.scenario,
                '-OuterDeadlineUtc', deadline]);
            assert.notEqual(result.status, 0); assert.match(result.stderr, /java\.exe/);
            assert.equal(existsSync(f.controllerPath), false);
        } finally { writeFileSync(f.controllerPath, controller, { flag: 'wx' }); }
        const authority = claimSeededLauncher(f, 'first-page-public-gates', false);
        assert.deepEqual(readBatchAuthorization(authority.source, { root: f.root, runID: authority.runID }).binding.ownedPids, [101]);
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
    } finally { f.close(); }
});
