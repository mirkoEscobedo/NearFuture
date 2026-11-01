// @ts-check
// The real admissions, claims, three providers, consent and ledger are exercised.
// Only OS effects, native observations and external provider closure are generated.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { completedSeedEscape } from './campaign-seed-exit-test-support.mjs';
import { seedInputSession } from './campaign-input-test-support.mjs';
import { json, filePin } from './seed-test-support.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';

/** @param {string} path */
const sha = path => filePin(path).sha256.toLowerCase();

/** @param {{confirmation?: boolean}} [options] */
export async function applicationQuitFixture(options = {}) {
    const f = await completedSeedEscape({ campaignControl: {
        schemaVersion: options.confirmation ? 7 : 6, profile: options.confirmation ?
            'DISPOSABLE_SEED_RETURN_MENU_QUIT_CONFIRMATION_NO_SAVE' : 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE' },
        maxScenarioActions: options.confirmation ? 9 : 8 });
    let h;
    try {
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID: f.runID }));
        /** @type {string[]} */ const effects = [];
        h = seedInputSession(f, tool => effects.push(tool), { window: () => f.window });
        const client = h;
        /** @param {number[]} coordinate */
        const click = coordinate => client.call('left_click', { target_window_id: 21, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate });
        /** Persist an original via the real framing API; expected scene constants are independent.
         * @param {'confirmation'|'menu'|'application-confirmation'} scene */
        async function inspect(scene) {
            await client.before();
            const args = { target_window_id: 21, quality: 0, width: 480 };
            const result = await client.call('screenshot', args), capturedAtMs = Date.now();
            const action = f.ledger().pending;
            const prefix = scene === 'confirmation' ? 'E2-no-save-return-confirmation' : scene === 'menu' ?
                'E2-returned-main-menu' : 'E2-application-quit-confirmation';
            const output = resolve(f.scenario, prefix); mkdirSync(output);
            const frame = persistDirectFrame({ output, label: 'capture-001', pngName: 'F1.original.png', result, args,
                window: f.window, ownedPids: [101], provenance: { runID: f.runID, batch: 'E2',
                    sourceSha256: sha(f.source), reservationID: action.reservationID, capturedAtMs } });
            assert.ok(frame.pngPath);
            const seed = JSON.parse(readFileSync(f.planPath, 'utf8')).seed;
            const receipt = { schemaVersion: 1,
                kind: scene === 'confirmation' ? 'STARSECTOR_NO_SAVE_RETURN_CONFIRMATION_INSPECTION' : scene === 'menu' ?
                    'STARSECTOR_RETURNED_MAIN_MENU_INSPECTION' : 'STARSECTOR_APPLICATION_QUIT_CONFIRMATION_INSPECTION',
                scenario: f.scenario, rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
                sourceSha256: sha(f.source), claimSha256: sha(resolve(f.scenario, 'guard-E2-consumed.private.json')),
                runID: f.runID, reservationID: action.reservationID,
                manifestSha256: seed.manifestSha256.toLowerCase(), stagingReceiptSha256: seed.stagingReceiptSha256.toLowerCase(),
                frame: { pngPath: frame.pngPath, pngSha256: sha(frame.pngPath), metadataPath: frame.metadataPath,
                    metadataSha256: sha(frame.metadataPath), capturedAtMs },
                inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                    ...(scene === 'confirmation' ? { state: 'RETURN_TO_MAIN_MENU_WITHOUT_SAVING',
                        dialogText: 'This will exit to the main menu WITHOUT saving your current game.',
                        confirmLabel: 'Ok', confirmCoordinate: [70, 110] }
                        : scene === 'menu' ? { state: 'STARSECTOR_MAIN_MENU_WITH_QUIT', quitLabel: 'Quit', quitCoordinate: [80, 120] }
                        : { state: 'EXIT_GAME_CONFIRMATION', dialogText: 'Exit game?', confirmLabel: 'Yes',
                            confirmCoordinate: [90, 130] }) } };
            const path = resolve(f.scenario, prefix + '-inspected.private.json'); json(path, receipt);
            await client.call('get_frontmost_app', {}); await client.call('get_window', { window_id: 21 });
            return { path, receipt, frame };
        }
        return { ...f, h: client, click, effects, inspect,
            close: async () => { await client.embedded.server.close(); await f.close(); } };
    } catch (error) { await h?.embedded.server.close(); await f.close(); throw error; }
}

/** @param {(f:Awaited<ReturnType<typeof applicationQuitFixture>>)=>Promise<void>} exercise
 * @param {{confirmation?: boolean}} [options] */
export async function withApplicationQuit(exercise, options = {}) {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    let f;
    try { f = await applicationQuitFixture(options); await exercise(f); }
    finally {
        await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
}
