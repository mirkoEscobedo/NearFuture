// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { writeFileSync, readFileSync, existsSync, rmSync } from 'node:fs';
import { resolve, basename, relative, isAbsolute } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { claimSeededLauncher, seedNativeSessionFactory } from './campaign-input-test-support.mjs';
import { directFixture } from '../direct-test-fixture.mjs';
import { startOwnedHttpBatch, requiredOwnedHttpFiles } from '../owned-http-batch.mjs';
import { json, filePin } from './seed-test-support.mjs';

test('seeded owned HTTP clips its original provider deadline and sends only guarded Play', async () => {
    const f = campaignLaunchFixture();
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof directFixture> | undefined} */ let provider;
    /** @type {Awaited<ReturnType<typeof startOwnedHttpBatch>> | undefined} */ let batch;
    let output = '';
    try {
        const powershell = resolve(f.scenario, 'synthetic-http-helper.exe');
        const supervisor = resolve(f.scenario, 'synthetic-http-verifier.exe');
        for (const path of [powershell, supervisor]) writeFileSync(path, 'Generated external process identity. Never executed.');
        const pinsPath = resolve(f.scenario, 'frozen-support-pins.private.json');
        json(pinsPath, { pins: [...new Set(requiredOwnedHttpFiles(f.root, powershell, supervisor))]
            .map(path => ({ path, sha256: filePin(path).sha256 })) });
        f.phaseLease.supportPinsSha256 = f.launchLease.supportPinsSha256 = filePin(pinsPath).sha256;
        const callerCap = Date.now() + 170000;
        f.plan.callerAdmissionDeadlineMs = f.launchLease.callerAdmissionDeadlineMs = callerCap;
        Object.assign(f.campaign.rootClock, { callerAdmissionDeadlineMs: callerCap,
            originalCallerAdmissionDeadlineMs: callerCap, effectiveDeadlineMs: callerCap });
        f.campaign.producerWitness.effectiveDeadlineMs = callerCap;
        f.refreshLaunch();
        const authority = claimSeededLauncher(f, 'synthetic-seed-http-' + basename(f.scenario), false);
        output = resolve(f.root, '.tmp/starsector-proxy', authority.runID);
        /** @type {string[]} */ const effects = [];
        provider = directFixture({ authority: { root: f.root, source: authority.source, runID: authority.runID,
            powershell, supervisor, consentMode: 'finite-explicit-consent' },
            sessionFactory: seedNativeSessionFactory(tool => effects.push(tool)) });
        batch = await startOwnedHttpBatch(provider.options, { spawn: provider.spawn });
        assert.equal(batch.deadline, callerCap - 30000);
        for (const [tool, args] of [['doctor', {}], ['policy_status', {}], ['get_window', { window_id: 19 }],
            ['screenshot', { target_window_id: 19, quality: 0, width: 480 }],
            ['get_frontmost_app', {}], ['get_window', { window_id: 19 }]]) {
            assert.notEqual((await batch.call(/** @type {string} */ (tool), args)).isError, true);
        }
        assert.notEqual((await batch.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] })).isError, true);
        assert.deepEqual(effects, ['left_click']);
        const state = JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
        assert.equal(state.history.reservedActions, 1); assert.equal(state.history.sentActions, 1);
        assert.equal(state.pending.batch, 'B1'); assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.equal(state.uncertain, false);
        const terminal = await batch.close();
        assert.equal(terminal.terminalObserved, true); assert.equal(terminal.cleanupUnknown, false);
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.terminalBoundToChild, true);
    } finally {
        await batch?.close(); await provider?.close();
        if (output && existsSync(output)) {
            const suffix = relative(resolve(f.root, '.tmp/starsector-proxy'), output);
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix)); rmSync(output, { recursive: true });
        }
        f.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
