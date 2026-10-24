// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { completedSeedEscape } from './campaign-seed-exit-test-support.mjs';
import { seedInputSession } from './campaign-input-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from '../batch-authorization.mjs';
import { readFileSync, appendFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

test('seeded B1 and inspected paused-entry Escape transfer into the same ordinary Exit without replenishing history', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedEscape>> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    try {
        f = await completedSeedEscape();
        const before = f.ledger();
        const admitted = readBatchAuthorization(f.source, { root: f.root, runID: f.runID });
        claimBatchAuthorization(admitted);
        assert.deepEqual(f.ledger().history, before.history);
        assert.equal(f.ledger().resolvedE1.reservationID, before.pending.reservationID);
        /** @type {string[]} */ const effects = [];
        const window = f.window;
        h = seedInputSession(f, tool => effects.push(tool), { window: () => window });
        await h.before();
        assert.notEqual((await h.call('left_click', { target_window_id: 21, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] })).isError, true);
        assert.deepEqual(effects, ['left_click']);
        const state = f.ledger();
        assert.equal(state.history.reservedActions, 6); assert.equal(state.history.sentActions, 6);
        assert.equal(state.resolvedB1.batch, 'B1'); assert.equal(state.resolvedE1.batch, 'E1');
        assert.equal(state.pending.batch, 'E2'); assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.equal(state.uncertain, false);
    } finally {
        await h?.embedded.server.close(); await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const point of ['before claim', 'after claim']) for (const name of ['E1-plan.private.json', 'E1-lease.private.json', 'B1-authorization.private.json',
    'B1-plan.private.json', 'E1-seed-paused/F1.original.png', 'E1-seed-paused/capture-001.raw.private.json',
    'E1-seed-paused/capture-001.metadata.private.json']) test('seeded Exit rechecks ' + name + ' ' + point, async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(key => process.env[key]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedEscape>> | undefined} */ let f;
    try {
        f = await completedSeedEscape();
        if (point === 'after claim') claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        appendFileSync(resolve(f.scenario, name), ' ');
        const path = resolve(f.scenario, 'guard-launch-history.private.json'), before = readFileSync(path), fixture = f;
        const read = point === 'after claim' ? readClaimedBatchAuthorization : readBatchAuthorization;
        assert.throws(() => read(fixture.source, { root: fixture.root, runID: fixture.runID }),
            /historical campaign|seeded B1 lineage|seeded B1 custody|paused seed|original PNG/);
        assert.deepEqual(readFileSync(path), before);
        assert.equal(existsSync(resolve(f.scenario, 'guard-E2-consumed.private.json')), point === 'after claim');
        assert.equal(f.ledger().history.reservedActions, 5);
        assert.equal(f.ledger().provider.batch, point === 'after claim' ? 'E2' : 'E1');
    } finally {
        await f?.close();
        names.forEach((key, i) => { if (old[i] === undefined) delete process.env[key]; else process.env[key] = old[i]; });
    }
});
