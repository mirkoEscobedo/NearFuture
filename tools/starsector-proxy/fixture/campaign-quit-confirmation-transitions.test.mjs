// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { withApplicationQuit } from './campaign-application-quit-test-support.mjs';

test('a separately inspected final Yes is the ninth input and remains pending ordinary-exit evidence', () =>
    withApplicationQuit(async f => {
        await f.h.before(); await f.click([40, 80]);
        await f.inspect('confirmation'); await f.click([70, 110]);
        await f.inspect('menu'); await f.click([80, 120]);
        assert.equal(f.ledger().history.reservedActions, 8);
        assert.equal(f.ledger().pending.status, 'AWAITING_AFTER');
        assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click']);
        await f.inspect('application-confirmation');
        assert.notEqual((await f.click([90, 130])).isError, true);
        const state = f.ledger();
        assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click', 'left_click']);
        assert.equal(state.history.reservedActions, 9); assert.equal(state.history.sentActions, 9);
        assert.equal(state.resolvedActions.length, 6); assert.equal(state.uncertain, false);
        assert.match(state.resolvedActions[5].applicationQuitConfirmationInspectionSha256, /^[a-f0-9]{64}$/);
        assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.deepEqual(state.pending.args.coordinate, [90, 130]);
        await assert.rejects(f.click([90, 130]), /mutation outside remaining sequence/);
        assert.equal(f.effects.length, 4); assert.equal(f.ledger().history.reservedActions, 9);
    }, { confirmation: true }));

test('closing the nine-input client after Quit sends no optional Yes and invents no final AFTER', async () => {
    /** @type {string[]|undefined} */ let effects;
    await withApplicationQuit(async f => {
        await f.h.before(); await f.click([40, 80]);
        await f.inspect('confirmation'); await f.click([70, 110]);
        await f.inspect('menu'); await f.click([80, 120]);
        effects = f.effects;
        const state = f.ledger();
        assert.equal(state.history.reservedActions, 8); assert.equal(state.history.sentActions, 8);
        assert.equal(state.resolvedActions.length, 5); assert.equal(state.uncertain, false);
        assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.deepEqual(state.pending.args.coordinate, [80, 120]);
    }, { confirmation: true });
    assert.deepEqual(effects, ['left_click', 'left_click', 'left_click']);
});
