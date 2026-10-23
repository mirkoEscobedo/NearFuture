// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, writeFileSync, unlinkSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from '../batch-authorization.mjs';
import { readFixtureBinding } from '../fixture-binding.mjs';
import { filePin, powershell } from './seed-test-support.mjs';
import { claimSeededLauncher } from './campaign-input-test-support.mjs';

test('protected seeded launch and phase-local B1 authority share an immutable launch predecessor', () => {
    const f = campaignLaunchFixture();
    try {
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const watchdog = f.watchdog();
        assert.equal(watchdog.status, 0, watchdog.stderr);
        assert.equal(watchdog.stdout.trim(), '480000');
        const launch = f.admit();
        assert.equal(launch.status, 0, launch.stderr);
        const options = { root: f.root, runID: f.runID };
        const authority = claimSeededLauncher(f, f.runID, false);
        const admitted = readBatchAuthorization(authority.source, options);
        assert.deepEqual(admitted.binding.ownedPids, [101]);
        assert.deepEqual(readFixtureBinding(admitted), { path: f.bindingPath,
            sha256: filePin(f.bindingPath).sha256.toLowerCase() });
        claimBatchAuthorization(admitted);
        assert.deepEqual(readClaimedBatchAuthorization(authority.source, options).binding, admitted.binding);
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
    } finally { f.close(); }
});

test('campaign control reaches both public launch gates over the same original pair before deliberately absent JRE effects', () => {
    const f = campaignLaunchFixture();
    try {
        assert.equal(existsSync(resolve(f.installation, 'jre/bin/java.exe')), false);
        const original = [f.planPath, f.leasePath].map(path => readFileSync(path));
        const controller = readFileSync(f.controllerPath); unlinkSync(f.controllerPath);
        const sentinel = resolve(f.scenario, 'launch-supervisor.private.json');
        writeFileSync(sentinel, 'Controlled pre-effect sentinel');
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
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), original);
        const authority = claimSeededLauncher(f, f.runID, false);
        assert.deepEqual(readBatchAuthorization(authority.source, { root: f.root, runID: f.runID }).binding.ownedPids, [101]);
    } finally { f.close(); }
});

for (const boundary of ['controller', 'caller']) test('self-consistent phase declarations cannot extend original ' + boundary + ' deadline', () => {
        const f = campaignLaunchFixture();
        try {
            if (boundary === 'controller') {
                f.controller.controllerDeadlineUtc = new Date(f.campaign.rootClock.effectiveDeadlineMs - 1000).toISOString();
            } else {
                const later = f.campaign.rootClock.deadlineMs;
                Object.assign(f.campaign.rootClock, { callerAdmissionDeadlineMs: later,
                    originalCallerAdmissionDeadlineMs: later, effectiveDeadlineMs: later });
                f.campaign.producerWitness.effectiveDeadlineMs = later;
            }
            f.refreshLaunch();
            const authority = claimSeededLauncher(f, f.runID, false);
            assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: f.runID }),
                /original caller admission|actual controller/, boundary);
        } finally { f.close(); }
});

test('phase clock uses the actual controller birth rather than its later publication timestamp', () => {
    const f = campaignLaunchFixture();
    try {
        f.controller.controllerCreationTime = new Date(Date.parse(f.controller.at) - 50).toISOString();
        f.campaign.producerWitness.controllerCreationTime = f.controller.controllerCreationTime;
        f.refreshLaunch();
        const authority = claimSeededLauncher(f, f.runID, false);
        assert.deepEqual(readBatchAuthorization(authority.source, { root: f.root, runID: f.runID }).binding.ownedPids, [101]);
    } finally { f.close(); }
});

test('unknown seed entry profile refuses the real protected launch and input boundaries', () => {
    const f = campaignLaunchFixture();
    try {
        f.plan.campaignControl.profile = 'NEW_GAME';
        f.launchLease.campaignControl.profile = 'NEW_GAME';
        f.refreshLaunch();
        const launch = f.admit();
        assert.notEqual(launch.status, 0, 'unknown profile must refuse before native launch effects');
        assert.match(launch.stderr, /campaign control/i);
        const authority = claimSeededLauncher(f, f.runID, false);
        assert.throws(() => readBatchAuthorization(authority.source, { root: f.root, runID: f.runID }), /seed entry launch profile/);
    } finally { f.close(); }
});
