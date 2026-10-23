// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { exitFixture, exitArgs } from './campaign-exit-test-fixture.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';

/** The clock override is confined to this isolated synthetic test process. Parent
 * Windows Job deadlines and actual native/host clocks are never changed.
 * @param {(f: ReturnType<typeof exitFixture>, realNow: () => number) => Promise<void>} action */
async function withScenario(action) {
    const realNow = Date.now;
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {ReturnType<typeof exitFixture> | undefined} */ let f;
    try { f = exitFixture(); await action(f, realNow); }
    finally {
        Date.now = realNow;
        await f?.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
        assert.equal(Date.now, realNow);
    }
}

test('E2 claim, consent and vendor Exit occur after known E1 expiry within the unchanged root', async () => withScenario(async (f, realNow) => {
    const completed = await f.completeEscape(), exit = f.stageExit(completed);
    const root = structuredClone(f.campaign.rootClock), future = completed.state.providerClock.deadlineMs + 1;
    assert.ok(future > realNow());
    assert.ok(root.effectiveDeadlineMs - root.cleanupReserveMs - future >= 120000);
    Date.now = () => future;
    const admitted = readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID });
    claimBatchAuthorization(admitted);
    assert.equal(f.history().providerClock.startMs, future);
    f.open(exit.source, exit.runID, () => {
        const state = f.history();
        assert.equal(Date.now(), future); assert.ok(future > completed.state.providerClock.deadlineMs);
        assert.equal(state.providerClock.startMs, future); assert.equal(state.pending.sentAtMs, future);
        assert.equal(state.history.reservedActions, 58); assert.equal(state.history.sentActions, 56);
        const current = readClaimedBatchAuthorization(exit.source, { root: f.root, runID: exit.runID });
        assert.ok('campaign' in current.binding);
        assert.deepEqual((/** @type {typeof f.campaign} */ (current.binding.campaign)).rootClock, root);
        assert.equal((/** @type {typeof f.campaign} */ (current.binding.campaign)).historyBaseline.reservedActions, 56);
        assert.equal((/** @type {typeof f.campaign} */ (current.binding.campaign)).maxReservedActions, 64);
    });
    await f.before();
    assert.notEqual((await f.call('left_click', exitArgs)).isError, true);
    assert.deepEqual(f.clicks, [[40, 80, 'left', 1]]);
    assert.equal(f.history().pending.knownResultAtMs, future);
    assert.equal(f.history().uncertain, false);
}));

test('E2 rechecks original root input slack after claim and BEFORE, even with provider time remaining', async () => withScenario(async f => {
    const completed = await f.completeEscape(), exit = f.stageExit(completed), root = f.campaign.rootClock;
    const tooLate = root.effectiveDeadlineMs - root.cleanupReserveMs - 120000 + 1;
    const claimTime = tooLate - 1001;
    assert.ok(claimTime > completed.state.providerClock.deadlineMs);
    Date.now = () => claimTime;
    claimBatchAuthorization(readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID }));
    f.open(exit.source, exit.runID); await f.before();
    assert.equal(f.history().providerClock.deadlineMs, root.effectiveDeadlineMs - root.cleanupReserveMs);
    Date.now = () => tooLate;
    assert.equal(f.history().providerClock.deadlineMs - Date.now(), 119999);
    assert.ok(f.history().providerClock.deadlineMs - Date.now() >= 20000);
    await assert.rejects(f.call('left_click', exitArgs), /original effective deadline/i);
    assert.deepEqual(f.moves, []); assert.deepEqual(f.clicks, []);
    assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
    assert.equal(f.history().uncertain, true);
}));
