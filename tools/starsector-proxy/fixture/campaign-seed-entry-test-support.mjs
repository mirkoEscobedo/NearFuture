// @ts-check
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync } from 'node:fs';
import { resolve, basename } from 'node:path';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { seedInputSession, seedTarget } from './campaign-input-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
import { claimBatchAuthorization, readBatchAuthorization } from '../batch-authorization.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';

/** Existing three-click entry is prerequisite setup, with generated observations. */
export async function enteredSeedFixture() {
    const f = await completedSeedLaunch();
    const runID = 'synthetic-paused-seed-entry-' + basename(f.scenario);
    claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID }));
    const window = { ...seedTarget, windowId: 21 };
    /** @type {string[]} */ const effects = [];
    const h = seedInputSession({ ...f, runID }, tool => effects.push(tool), { window: () => window });
    async function close() { await h.embedded.server.close(); await f.close(); }
    try {
        for (const coordinate of [[40, 80], [50, 90], [60, 100]]) {
            await h.before();
            assert.notEqual((await h.call('left_click', { target_window_id: 21, target_app: 'java.exe',
                focus_strategy: 'strict', coordinate })).isError, true);
        }
        assert.deepEqual(effects, ['left_click', 'left_click', 'left_click']);
        async function inspectPaused() {
            await h.before();
            const args = { target_window_id: 21, quality: 0, width: 480 };
            const result = await h.call('screenshot', args), capturedAtMs = Date.now();
            const action = f.ledger().pending;
            const output = resolve(f.scenario, 'E1-seed-paused'); mkdirSync(output);
            const frame = persistDirectFrame({ output, label: 'capture-001', pngName: 'F1.original.png',
                result, args, window, ownedPids: [101], provenance: { runID, batch: 'E1',
                    sourceSha256: filePin(f.source).sha256.toLowerCase(), reservationID: action.reservationID, capturedAtMs } });
            assert.ok(frame.pngPath);
            const seed = JSON.parse(readFileSync(f.planPath, 'utf8')).seed;
            const receipt = { schemaVersion: 1, kind: 'STARSECTOR_SEED_PAUSE_INSPECTION', scenario: f.scenario,
                rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: filePin(f.controllerPath).sha256.toLowerCase(),
                sourceSha256: filePin(f.source).sha256.toLowerCase(),
                claimSha256: filePin(resolve(f.scenario, 'guard-E1-consumed.private.json')).sha256.toLowerCase(),
                runID, reservationID: action.reservationID,
                manifestSha256: seed.manifestSha256.toLowerCase(), stagingReceiptSha256: seed.stagingReceiptSha256.toLowerCase(),
                frame: { pngPath: frame.pngPath, pngSha256: filePin(frame.pngPath).sha256.toLowerCase(),
                    metadataPath: frame.metadataPath, metadataSha256: filePin(frame.metadataPath).sha256.toLowerCase(), capturedAtMs },
                inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                    state: 'INTENDED_DISPOSABLE_SEED_PAUSED' } };
            const path = resolve(f.scenario, 'E1-seed-paused-inspected.private.json'); json(path, receipt);
            await h.call('get_frontmost_app', {}); await h.call('get_window', { window_id: 21 });
            return { path, receipt };
        }
        return { ...f, runID, h, effects, window, inspectPaused, close };
    } catch (error) { await close(); throw error; }
}
