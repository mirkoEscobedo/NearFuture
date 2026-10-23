// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { enteredSeedFixture } from './campaign-seed-entry-test-support.mjs';
import { json } from './seed-test-support.mjs';

test('manifest-bound inspected original paused seed admits exactly one Escape in the original cumulative history', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof enteredSeedFixture>> | undefined} */ let f;
    try {
        f = await enteredSeedFixture();
        const before = f.ledger(), clock = before.providerClock;
        await f.inspectPaused();
        const args = { text: 'escape', repeat: 1, target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict' };
        const result = await f.h.call('key', args);
        assert.notEqual(result.isError, true, JSON.stringify(result));
        assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click', 'key']);
        const state = f.ledger();
        assert.equal(state.history.reservedActions, 5); assert.equal(state.history.sentActions, 5);
        assert.equal(state.resolvedActions.length, 3);
        assert.equal(state.pending.tool, 'key'); assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.equal(state.uncertain, false); assert.deepEqual(state.providerClock, clock);
        const fixture = f;
        await assert.rejects(() => fixture.h.call('key', args), /mutation outside remaining sequence/);
        assert.equal(f.ledger().history.reservedActions, 5);
        assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click', 'key']);
    } finally {
        await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const fault of ['another seed', 'another reservation', 'another claim', 'different PNG', 'unpaused scene'])
    test('seed Escape refuses inspected proof for ' + fault + ' without a key effect or action refund', async () => {
        const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
        const old = names.map(name => process.env[name]);
        process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
        process.env.COMPUTER_USE_AUDIT_LOG = 'false';
        /** @type {Awaited<ReturnType<typeof enteredSeedFixture>> | undefined} */ let f;
        try {
            f = await enteredSeedFixture();
            const proof = await f.inspectPaused();
            if (fault === 'another seed') proof.receipt.manifestSha256 = '0'.repeat(64);
            if (fault === 'another reservation') proof.receipt.reservationID += '-changed';
            if (fault === 'another claim') proof.receipt.claimSha256 = '0'.repeat(64);
            if (fault === 'different PNG') proof.receipt.frame.pngSha256 = '0'.repeat(64);
            if (fault === 'unpaused scene') proof.receipt.inspection.state = 'UNPAUSED';
            json(proof.path, proof.receipt);
            const fixture = f;
            await assert.rejects(() => fixture.h.call('key', { text: 'escape', repeat: 1, target_window_id: 21,
                target_app: 'java.exe', focus_strategy: 'strict' }),
            /paused seed inspection lineage|paused seed original custody|paused seed visible inspection/);
            assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click']);
            const state = f.ledger();
            assert.equal(state.history.reservedActions, 4); assert.equal(state.history.sentActions, 4);
            assert.equal(state.pending.tool, 'left_click'); assert.equal(state.pending.status, 'AWAITING_AFTER');
            assert.equal(state.resolvedActions.length, 2); assert.equal(state.uncertain, true);
        } finally {
            await f?.close();
            names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
        }
    });
