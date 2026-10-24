// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { completedSeedEscape } from './campaign-seed-exit-test-support.mjs';
import { seedInputSession } from './campaign-input-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';
/** @param {string} path */
const sha = path => filePin(path).sha256.toLowerCase();

/** @param {string} fault */
async function confirmationScenario(fault) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedEscape>> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    try {
        f = await completedSeedEscape();
        const auth = JSON.parse(readFileSync(f.source, 'utf8'));
        const declaration = fault === 'undeclared confirmation' ? undefined :
            { schemaVersion: 1, dialogText: fault === 'unknown declaration' ? 'Save game?' : 'Exit game?', confirmLabel: 'Yes' };
        const plan = { ...f.plan, exitConfirmation: declaration };
        json(auth.planPath, plan);
        json(auth.leasePath, { ...f.lease, planSha256: sha(auth.planPath), exitConfirmation: declaration });
        json(f.source, { ...auth, planSha256: sha(auth.planPath), leaseSha256: sha(auth.leasePath) });
        if (fault === 'unknown declaration') {
            const fixture = f;
            assert.throws(() => readBatchAuthorization(fixture.source, { root: fixture.root, runID: fixture.runID }),
                /closed matching exit confirmation declaration/);
            assert.equal(f.ledger().provider.batch, 'E1'); assert.equal(f.ledger().history.reservedActions, 5);
            return;
        }
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        /** @type {string[]} */ const effects = [];
        const window = f.window;
        h = seedInputSession(f, tool => effects.push(tool), { window: () => window });
        await h.before();
        const click = { target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict', coordinate: [40, 80] };
        assert.notEqual((await h.call('left_click', click)).isError, true);
        await h.before();
        const args = { target_window_id: 21, quality: 0, width: 480 };
        const result = await h.call('screenshot', args), capturedAtMs = Date.now();
        const action = f.ledger().pending;
        const output = resolve(f.scenario, 'E2-exit-confirmation'); mkdirSync(output);
        const frame = persistDirectFrame({ output, label: 'capture-001', pngName: 'F1.original.png', result, args, window,
            ownedPids: [101], provenance: { runID: f.runID, batch: 'E2', sourceSha256: sha(f.source),
                reservationID: action.reservationID, capturedAtMs } }); assert.ok(frame.pngPath);
        const seed = JSON.parse(readFileSync(f.planPath, 'utf8')).seed;
        const receipt = { schemaVersion: 1, kind: 'STARSECTOR_EXIT_CONFIRMATION_INSPECTION', scenario: f.scenario,
            rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
            sourceSha256: sha(f.source), claimSha256: sha(resolve(f.scenario, 'guard-E2-consumed.private.json')),
            runID: f.runID, reservationID: action.reservationID, manifestSha256: seed.manifestSha256.toLowerCase(),
            stagingReceiptSha256: seed.stagingReceiptSha256.toLowerCase(),
            frame: { pngPath: frame.pngPath, pngSha256: sha(frame.pngPath), metadataPath: frame.metadataPath,
                metadataSha256: sha(frame.metadataPath), capturedAtMs },
            inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                state: 'EXIT_GAME_CONFIRMATION', dialogText: 'Exit game?', confirmLabel: 'Yes', confirmCoordinate: [70, 110] } };
        if (fault === 'different dialog') receipt.inspection.dialogText = 'Save game?';
        if (fault === 'different button') receipt.inspection.confirmLabel = 'Save';
        if (fault === 'another reservation') receipt.reservationID += '-changed';
        json(resolve(f.scenario, 'E2-exit-confirmation-inspected.private.json'), receipt);
        await h.call('get_frontmost_app', {}); await h.call('get_window', { window_id: 21 });
        if (fault !== 'none') {
            const client = h;
            await assert.rejects(() => client.call('left_click', { ...click,
                coordinate: fault === 'different coordinate' ? [71, 110] : [70, 110] }),
            /mutation outside remaining sequence|matching declared exit confirmation|exit confirmation inspection lineage/);
            assert.deepEqual(effects, ['left_click']);
            const state = f.ledger();
            assert.equal(state.history.reservedActions, 6); assert.equal(state.history.sentActions, 6);
            assert.equal(state.resolvedActions.length, 3); assert.equal(state.pending.status, 'AWAITING_AFTER');
            assert.equal(state.uncertain, true);
            return;
        }
        assert.notEqual((await h.call('left_click', { ...click, coordinate: [70, 110] })).isError, true);
        assert.deepEqual(effects, ['left_click', 'left_click']);
        const state = f.ledger();
        assert.equal(state.history.reservedActions, 7); assert.equal(state.history.sentActions, 7);
        assert.equal(state.resolvedActions.length, 4);
        assert.match(state.resolvedActions[3].exitConfirmationInspectionSha256, /^[a-f0-9]{64}$/);
        assert.equal(state.pending.batch, 'E2'); assert.equal(state.pending.status, 'AWAITING_AFTER');
        assert.equal(state.uncertain, false);
    } finally {
        await h?.embedded.server.close(); await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
}

test('seeded E2 permits one separately declared and inspected Exit game Yes confirmation in the same global bound',
    () => confirmationScenario('none'));
for (const fault of ['undeclared confirmation', 'unknown declaration', 'different dialog', 'different button',
    'different coordinate', 'another reservation']) test('seeded E2 refuses ' + fault + ' without another effect',
    () => confirmationScenario(fault));
