// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { withApplicationQuit } from './campaign-application-quit-test-support.mjs';

test('eight known inputs reach the main-menu Quit and retain its unresolved final outcome', () => withApplicationQuit(async f => {
    await f.h.before(); await f.click([40, 80]);
    const confirmation = await f.inspect('confirmation');
    assert.notEqual((await f.click([70, 110])).isError, true);
    const returnedMenu = await f.inspect('menu');
    assert.notEqual((await f.click([80, 120])).isError, true);
    assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click']);
    const state = f.ledger();
    assert.equal(state.history.reservedActions, 8); assert.equal(state.history.sentActions, 8);
    assert.equal(state.resolvedActions.length, 5); assert.equal(state.uncertain, false);
    assert.match(state.resolvedActions[3].noSaveReturnInspectionSha256, /^[a-f0-9]{64}$/);
    assert.match(state.resolvedActions[4].returnedMainMenuInspectionSha256, /^[a-f0-9]{64}$/);
    assert.notEqual(confirmation.path, returnedMenu.path);
    assert.equal(state.pending.batch, 'E2'); assert.equal(state.pending.status, 'AWAITING_AFTER');
    assert.deepEqual(state.pending.args.coordinate, [80, 120]);
    await f.h.before();
    await assert.rejects(f.click([80, 120]), /mutation outside remaining sequence/);
    assert.deepEqual(f.effects, ['left_click', 'left_click', 'left_click']);
    assert.equal(f.ledger().history.reservedActions, 8);
}));
