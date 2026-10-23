// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { json, filePin } from './seed-test-support.mjs';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from '../batch-authorization.mjs';

test('known closed seeded B1 transfers the replacement game HWND and unchanged global history into E1', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    try {
        f = await completedSeedLaunch();
        const before = f.ledger();
        const options = { root: f.root, runID: 'seed-entry-after-known-launch' };
        const admitted = readBatchAuthorization(f.source, options);
        assert.equal(admitted.binding.batch, 'E1');
        assert.equal(admitted.binding.schemaVersion, 3);
        assert.ok('campaign' in admitted.binding);
        assert.equal(admitted.binding.campaign.windowId, 21);
        assert.deepEqual(admitted.binding.ownedPids, [101]);
        claimBatchAuthorization(admitted);
        assert.deepEqual(readClaimedBatchAuthorization(f.source, options).binding, admitted.binding);
        const state = f.ledger();
        assert.deepEqual(state.history, before.history);
        assert.equal(state.resolvedB1.reservationID, before.pending.reservationID);
        assert.equal(state.pending, null);
        assert.equal(state.provider.batch, 'E1');
        assert.equal(state.provider.runID, options.runID);
        assert.equal(state.uncertain, false);
        const claimedHistory = readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'));
        assert.throws(() => claimBatchAuthorization(admitted), /EEXIST/);
        assert.deepEqual(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json')), claimedHistory);
        assert.deepEqual([f.planPath, f.leasePath].map(path => readFileSync(path)), f.originalLaunchBytes);
        assert.deepEqual(f.effects, ['left_click']);
    } finally {
        await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const fault of ['unknown provider closure', 'changed process birth', 'foreign PID', 'different HWND',
    'wrong original PNG', 'changed B1 claim']) test('seeded E1 refuses ' + fault + ' before ownership transfer', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    try {
        f = await completedSeedLaunch();
        if (fault === 'unknown provider closure') {
            f.terminal.cleanupUnknown = true; json(f.terminalPath, f.terminal);
            f.receipt.provider.terminalSha256 = filePin(f.terminalPath).sha256.toLowerCase();
        }
        if (fault === 'changed process birth' || fault === 'foreign PID') {
            const path = f.auth.ownedReceiptPath, owned = JSON.parse(readFileSync(path, 'utf8'));
            if (fault === 'changed process birth') owned.processes[0].creationTime =
                new Date(Date.parse(owned.processes[0].creationTime) + 1).toISOString();
            else { owned.processes[0].pid = 102; owned.processes[0].supervisorJobPids = [100, 102]; }
            json(path, owned); f.auth.ownedReceiptSha256 = filePin(path).sha256;
        }
        if (fault === 'different HWND') f.plan.campaign.windowId = f.lease.campaign.windowId = 22;
        if (fault === 'wrong original PNG') f.receipt.frame.pngSha256 = '0'.repeat(64);
        if (fault === 'changed B1 claim') {
            const state = f.ledger(); state.b1ClaimSha256 = '0'.repeat(64);
            json(resolve(f.scenario, 'guard-launch-history.private.json'), state);
        }
        json(f.receiptPath, f.receipt);
        f.plan.b1MenuReceipt.sha256 = f.lease.b1MenuReceipt.sha256 = filePin(f.receiptPath).sha256.toLowerCase();
        f.lease.planSha256 = f.auth.planSha256 = json(f.phasePlanPath, f.plan).sha256;
        f.auth.leaseSha256 = json(f.phaseLeasePath, f.lease).sha256; json(f.source, f.auth);
        const historyPath = resolve(f.scenario, 'guard-launch-history.private.json'), before = readFileSync(historyPath);
        const fixture = f;
        assert.throws(() => readBatchAuthorization(fixture.source, { root: fixture.root, runID: 'refused-seed-entry' }),
            /known provider closure|seeded owned process lineage|seeded main-menu original custody|original seeded B1 lineage/);
        assert.deepEqual(readFileSync(historyPath), before);
        assert.deepEqual(f.effects, ['left_click']);
        assert.equal(f.ledger().provider.batch, 'B1');
    } finally {
        await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});
