// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { exitFixture, exitArgs } from './campaign-exit-test-fixture.mjs';
import { sha } from './campaign-test-fixture.mjs';
import { write } from './batch-test-fixture.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';

/** @param {(f: ReturnType<typeof exitFixture>) => Promise<void>} action */
async function scenario(action) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    const f = exitFixture();
    try { await action(f); }
    finally {
        await f.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
    }
}

test('known closed E1 and inspected original menu transfer the same launch to one ordinary Exit click', async () => scenario(async f => {
    const completed = await f.completeEscape(), exit = f.stageExit(completed);
    const initialClaim = JSON.parse(readFileSync(resolve(f.scenario, 'guard-E1-consumed.private.json'), 'utf8'));
    const root = initialClaim.binding.campaign.rootClock;
    // Historical E1 proof remains valid after its known-closed interval, without
    // turning that interval into a new root or demanding live E1 time remaining.
    const afterE1Deadline = completed.state.providerClock.deadlineMs + 1;
    const admitted = readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID, now: afterE1Deadline });
    assert.ok('campaign' in admitted.binding);
    assert.deepEqual(admitted.binding.campaign, initialClaim.binding.campaign);
    claimBatchAuthorization(admitted);
    f.open(exit.source, exit.runID, () => {
        const state = f.history();
        assert.equal(state.history.reservedActions, 58); assert.equal(state.history.sentActions, 56);
        assert.equal(state.uncertain, false); assert.equal(state.pending.batch, 'E2');
        assert.equal(state.pending.runID, exit.runID); assert.equal(state.pending.sourceSha256, sha(exit.source));
        assert.equal(state.resolvedE1.reservationID, completed.receipt.e1.reservationID);
        assert.equal(state.resolvedE1.menuReceiptSha256, sha(exit.menuPath));
        assert.ok(state.providerClock.deadlineMs <= root.effectiveDeadlineMs - root.cleanupReserveMs);
        const current = readClaimedBatchAuthorization(exit.source, { root: f.root, runID: exit.runID });
        assert.ok('campaign' in current.binding);
        assert.deepEqual(current.binding.campaign, initialClaim.binding.campaign);
        assert.equal(current.binding.campaign.historyBaseline.reservedActions, 56);
        assert.equal(current.binding.campaign.maxReservedActions, 64);
        assert.equal(sha(f.source), completed.originalE1SourceSha256);
    });
    await f.before();
    const result = await f.call('left_click', exitArgs);
    assert.notEqual(result.isError, true); assert.match(JSON.stringify(result), /Clicked \(40, 80\)/);
    assert.deepEqual(f.keys, [['escape', 1]]);
    assert.deepEqual(f.moves, [[40, 80]]); assert.deepEqual(f.clicks, [[40, 80, 'left', 1]]);
    assert.equal(f.history().pending.status, 'AWAITING_AFTER');
    assert.equal(f.history().history.reservedActions, 58); assert.equal(f.history().history.sentActions, 56);
    assert.equal(f.history().uncertain, false);
}));

/** @type {[string, RegExp][]} */
const refusals = [
    ['changed-root-clock', /initial campaign lineage/i],
    ['reset-baseline-and-limit', /initial campaign lineage/i],
    ['wrong-reservation', /menu reservation proof/i],
    ['wrong-original-image', /menu frame custody/i],
    ['unknown-provider-close', /known provider closure/i],
    ['failed-provider-close', /known provider closure/i],
];

for (const [fault, reason] of refusals) test('E2 handoff refuses ' + fault + ' without refund, replacement owner or effect', async () => scenario(async f => {
    const completed = await f.completeEscape();
    if (fault === 'changed-root-clock') {
        f.plan.campaign = structuredClone(f.plan.campaign); f.plan.campaign.rootClock.startMs++;
    }
    if (fault === 'reset-baseline-and-limit') {
        f.plan.campaign = structuredClone(f.plan.campaign);
        f.plan.campaign.historyBaseline.reservedActions = 57; f.plan.campaign.maxReservedActions = 65;
    }
    if (fault === 'wrong-reservation') completed.receipt.e1.reservationID += '-another-action';
    if (fault === 'wrong-original-image') completed.receipt.frame.pngSha256 = '0'.repeat(64);
    if (fault === 'unknown-provider-close' || fault === 'failed-provider-close') {
        if (fault === 'unknown-provider-close') {
            completed.terminal.terminalObserved = false; completed.terminal.cleanupUnknown = true;
        } else completed.terminal.closeFailed = true;
        write(completed.terminalPath, completed.terminal);
        completed.receipt.provider.terminalSha256 = sha(completed.terminalPath);
    }
    const exit = f.stageExit(completed), historyBefore = readFileSync(f.historyPath);
    assert.throws(() => readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID }), reason);
    assert.ok(readFileSync(f.historyPath).equals(historyBefore));
    assert.equal(existsSync(resolve(f.scenario, 'guard-E2-consumed.private.json')), false);
    assert.deepEqual(f.keys, [['escape', 1]]); assert.deepEqual(f.moves, []); assert.deepEqual(f.clicks, []);
    assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
}));
