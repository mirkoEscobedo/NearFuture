// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { createBatchSession } from './batch-session.mjs';
import { handleWindowTool } from '../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/session/window-handlers.js';
const auth = { schemaVersion: 1, batch: 'B1', runID: 'synthetic-run', scenario: 'synthetic', planSha256: 'a'.repeat(64), ownedPids: [101] };
const activate = { window_id: 7 };
const click = { target_window_id: 7, focus_strategy: 'strict', coordinate: [15, 20] };
const target = { pid: 101, windowId: 7, title: 'Starsector', bundleId: 'java.exe', isFocused: true, bounds: { x: 0, y: 0, width: 100, height: 80 } };
const image = { content: [{ type: 'image', mimeType: 'image/png', data: 'original-bytes' }] };
/** @param {any} value */
const json = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
/** @typedef {(tool: string, args: any, signal?: AbortSignal, progress?: any, context?: any) => Promise<any>} Dispatch */
/** @param {Dispatch} [inspect] */
function backend(inspect = async () => undefined) {
    /** @type {Dispatch} */
    const dispatch = async (tool, args, signal, progress, context) => {
        const result = await inspect(tool, args, signal, progress, context);
        if (result !== undefined) return result;
        if (tool === 'doctor') return json({ ok: true });
        if (tool === 'policy_status') return json({ profile: 'core' });
        if (tool === 'get_window') return json(target);
        if (tool === 'get_frontmost_app') return handleWindowTool(tool, args, /** @type {any} */ ({ native: { getFrontmostApp: () => ({ pid: 101, bundleId: 'java.exe', displayName: 'Synthetic Starsector' }) } }));
        if (tool === 'screenshot') return image;
        if (tool === 'activate_window') return json({ windowId: 7, activated: true });
        return json({ ok: true });
    };
    return { dispatch, getLastScreenshot: () => image };
}
/** @param {any} guard */
async function before(guard) {
    await guard.dispatch('doctor', {}); await guard.dispatch('policy_status', {});
    await guard.dispatch('get_window', { window_id: 7 });
    await guard.dispatch('screenshot', { target_window_id: 7, quality: 0, width: 100 });
    await guard.dispatch('get_frontmost_app', {}); await guard.dispatch('get_window', { window_id: 7 });
}

test('actual pinned foreground handler wrapper permits a fresh strict owned click', async () => {
    let clicks = 0;
    const guard = createBatchSession(backend(async tool => { if (tool === 'left_click') clicks++; }), auth);
    await before(guard); await guard.dispatch('left_click', click); assert.equal(clicks, 1);
});

test('flat, null, malformed or foreign foreground receipts never authorize a strict click', async () => {
    for (const receipt of [{ pid: 101 }, { app: null }, { app: {} }, { app: { pid: '101', bundleId: 'java.exe', displayName: 'Synthetic' } },
        { app: { pid: 999, bundleId: 'java.exe', displayName: 'Synthetic' } }, { app: { pid: 101 } }]) {
        let clicks = 0;
        const guard = createBatchSession(backend(async tool => {
            if (tool === 'get_frontmost_app') return json(receipt); if (tool === 'left_click') clicks++;
        }), auth);
        await before(guard); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(clicks, 0);
    }
});
test('complete native calls serialize and preserve signal, context and original image identity', async () => {
    let active = 0, maximum = 0;
    const controller = new AbortController(), progress = () => {}, context = { elicitApproval: async () => false, clientRoots: ['unchanged'] };
    const args = { width: 100, quality: 0, target_window_id: 7 };
    const guard = createBatchSession(backend(async (tool, actualArgs, signal, actualProgress, actualContext) => {
        active++; maximum = Math.max(maximum, active);
        await new Promise(resolve => setTimeout(resolve, 5)); active--;
        if (actualArgs === args) { assert.equal(signal, controller.signal); assert.equal(actualProgress, progress); assert.equal(actualContext, context); }
    }), auth);
    await before(guard);
    const results = await guard.runRequest('one registered handler', async () => Promise.all([
        guard.dispatch('screenshot', args, controller.signal, progress, context), guard.dispatch('screenshot', args, controller.signal, progress, context)]));
    assert.equal(maximum, 1); assert.ok(results.every(/** @param {any} value */ value => value === image)); assert.equal(guard.getLastScreenshot(), image);
});
test('duplicate activation denies itself and an already queued Play before native dispatch', async () => {
    /** @type {string[]} */ const actions = [];
    const guard = createBatchSession(backend(async tool => { if (['activate_window', 'left_click'].includes(tool)) actions.push(tool); }), auth);
    await before(guard);
    const results = await guard.runRequest('one registered handler', async () => Promise.allSettled([
        guard.dispatch('activate_window', activate), guard.dispatch('activate_window', activate), guard.dispatch('left_click', click)]));
    assert.deepEqual(results.map(/** @param {any} value */ value => value.status), ['fulfilled', 'rejected', 'rejected']); assert.deepEqual(actions, ['activate_window']);
});
test('exhausted sequence never restarts', async () => {
    /** @type {string[]} */ const actions = [];
    const guard = createBatchSession(backend(async tool => { if (['activate_window', 'left_click'].includes(tool)) actions.push(tool); }), auth);
    await before(guard); await guard.dispatch('left_click', click);
    await assert.rejects(guard.dispatch('activate_window', activate)); await assert.rejects(guard.dispatch('left_click', click)); assert.deepEqual(actions, ['left_click']);
});
test('queued cancelled input skips native dispatch and latches later input', async () => {
    /** @type {() => void} */ let release = () => {};
    const waiting = new Promise(resolve => { release = () => resolve(undefined); });
    let actions = 0;
    const guard = createBatchSession(backend(async tool => { if (tool === 'wait') await waiting; if (['activate_window', 'left_click'].includes(tool)) actions++; }), auth);
    await before(guard);
    await guard.runRequest('one registered handler', async () => {
        const first = guard.dispatch('wait', {}); const controller = new AbortController();
        const second = guard.dispatch('activate_window', activate, controller.signal); controller.abort(); release();
        await first; await assert.rejects(second);
    });
    await assert.rejects(guard.dispatch('left_click', click)); assert.equal(actions, 0);
});
for (const failure of ['denial', 'throw']) test(failure + ' consumes attempt, releases queue and preserves input latch', async () => {
    let actions = 0; const denied = { isError: true, content: [{ type: 'text', text: 'native policy denied' }] };
    const guard = createBatchSession(backend(async tool => { if (tool !== 'activate_window' && tool !== 'left_click') return; actions++; if (failure === 'throw') throw new Error('native failure'); return denied; }), auth);
    await before(guard);
    if (failure === 'throw') await assert.rejects(guard.dispatch('activate_window', activate)); else assert.equal(await guard.dispatch('activate_window', activate), denied);
    await assert.rejects(guard.dispatch('left_click', click)); await before(guard); assert.equal(actions, 1);
});
test('malformed bindings and unlisted input fail closed', async () => {
    for (const bad of [undefined, {}, { ...auth, batch: 'unknown' }, { ...auth, batch: ['B1'] }, { ...auth, ownedPids: [] }, { ...auth, extra: true }]) assert.throws(() => createBatchSession(backend(), bad));
    let actions = 0; const guard = createBatchSession(backend(async tool => { if (tool === 'type' || tool === 'left_click') actions++; }), auth);
    await before(guard); await assert.rejects(guard.dispatch('type', { text: 'unlisted' })); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(actions, 0);
});
test('activation with known target but no BEFORE PNG never dispatches', async () => {
    let actions = 0; const guard = createBatchSession(backend(async tool => { if (tool === 'activate_window') actions++; }), auth);
    await guard.dispatch('doctor', {}); await guard.dispatch('policy_status', {}); await guard.dispatch('get_window', { window_id: 7 });
    await assert.rejects(guard.dispatch('activate_window', activate)); assert.equal(actions, 0);
});
test('wrong target and non-strict input never reach native', async () => {
    for (const scenario of ['wrong-target', 'non-strict']) {
        let actions = 0; const guard = createBatchSession(backend(async tool => { if (tool === 'left_click') actions++; }), auth);
        await before(guard);
        if (scenario === 'wrong-target') await assert.rejects(guard.dispatch('screenshot', { target_window_id: 999, quality: 0, width: 100 }));
        await assert.rejects(guard.dispatch('left_click', { ...click, focus_strategy: 'best_effort' })); assert.equal(actions, 0);
    }
});
test('activation requires a new original and fresh focus before Play', async () => {
    let clicks = 0; const guard = createBatchSession(backend(async tool => { if (tool === 'left_click') clicks++; }), auth);
    await before(guard); await guard.dispatch('activate_window', activate); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(clicks, 0);
});
test('a strict click outside the owned window or aimed at another window never dispatches', async () => {
    for (const args of [{ ...click, coordinate: [101, 20] }, { ...click, target_window_id: 999 }]) {
        let clicks = 0;
        const guard = createBatchSession(backend(async tool => { if (tool === 'left_click') clicks++; }), auth);
        await before(guard); await assert.rejects(guard.dispatch('left_click', args)); assert.equal(clicks, 0);
    }
});
test('an activation semantic failure preserves its result and blocks later Play after new receipts', async () => {
    for (const ack of [{ windowId: 7, activated: false, reason: 'raise_failed' }, { windowId: 999, activated: true }, {}]) {
        let clicks = 0; const result = json(ack);
        const guard = createBatchSession(backend(async tool => {
            if (tool === 'activate_window') return result; if (tool === 'left_click') clicks++;
        }), auth);
        await before(guard); assert.equal(await guard.dispatch('activate_window', activate), result);
        await before(guard); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(clicks, 0);
    }
});
test('malformed focus receipts cannot authorize a strict click', async () => {
    let clicks = 0;
    const guard = createBatchSession(backend(async tool => {
        if (tool === 'get_window') return json({ ...target, isFocused: 'false' });
        if (tool === 'left_click') clicks++;
    }), auth);
    await before(guard); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(clicks, 0);
});
test('valid three-batch sequences remain bounded and preserve native result identity', async () => {
    for (const [batch, tools] of [['B1', ['activate_window', 'left_click']], ['B2', ['activate_window', 'mouse_move', 'mouse_move']], ['B3', ['left_click', 'left_click']]]) {
        /** @type {string[]} */ const actions = []; const result = json({ native: 'unchanged' });
        const activated = json({ windowId: 7, activated: true });
        const guard = createBatchSession(backend(async tool => { if (['activate_window', 'left_click', 'mouse_move'].includes(tool)) { actions.push(tool); return tool === 'activate_window' ? activated : result; } }), { ...auth, batch });
        for (const tool of tools) { await before(guard); assert.equal(await guard.dispatch(tool, tool === 'activate_window' ? activate : click), tool === 'activate_window' ? activated : result); }
        await before(guard); await assert.rejects(guard.dispatch('left_click', click)); assert.deepEqual(actions, tools);
    }
});

test('overlapping request submission fails the batch even when native operations could have queued', async () => {
    let actions = 0;
    /** @type {Record<string, unknown>[]} */ const records = [];
    const guard = createBatchSession(backend(async tool => {
        if (tool === 'wait') await new Promise(resolve => setTimeout(resolve, 5));
        if (tool === 'activate_window') actions++;
    }), auth, event => records.push(event));
    const first = guard.dispatch('wait', {});
    await assert.rejects(guard.dispatch('policy_status', {}), /Concurrent/); await first;
    await before(guard); await assert.rejects(guard.dispatch('activate_window', activate), /latched/);
    assert.equal(actions, 0); assert.ok(records.some(event => event.phase === 'request-overlap'));
});

test('overlap cannot undo an already running action and blocks every later input', async () => {
    /** @type {() => void} */ let started = () => {}, release = () => {};
    const begun = new Promise(resolve => { started = () => resolve(undefined); });
    const held = new Promise(resolve => { release = () => resolve(undefined); });
    let actions = 0;
    const guard = createBatchSession(backend(async tool => {
        if (tool === 'activate_window') { actions++; started(); await held; }
        if (tool === 'left_click') actions++;
    }), auth);
    await before(guard); const first = guard.dispatch('activate_window', activate); await begun;
    await assert.rejects(guard.dispatch('policy_status', {}), /Concurrent/); release(); await first;
    await before(guard); await assert.rejects(guard.dispatch('left_click', click)); assert.equal(actions, 1);
});
