// @ts-check
// Real earlier input phases are setup; OS provider closure remains generated.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync, rmSync, realpathSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { enteredSeedFixture } from './campaign-seed-entry-test-support.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';
import { json, filePin as rawPin } from './seed-test-support.mjs';
/** @param {string} path */
const sha = path => rawPin(path).sha256.toLowerCase();

export async function completedSeedEscape() {
    const f = await enteredSeedFixture();
    const outputParent = resolve(f.root, '.tmp/starsector-proxy'), output = resolve(outputParent, f.runID);
    let outputCreated = false;
    async function close() {
        if (outputCreated) {
            const suffix = relative(outputParent, output);
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix)); rmSync(output, { recursive: true });
        }
        await f.close();
    }
    try {
        await f.inspectPaused();
        assert.notEqual((await f.h.call('key', { text: 'escape', repeat: 1, target_window_id: 21,
            target_app: 'java.exe', focus_strategy: 'strict' })).isError, true);
        await f.h.call('get_window', { window_id: 21 });
        const args = { target_window_id: 21, quality: 0, width: 480 };
        const result = await f.h.call('screenshot', args), capturedAtMs = Date.now();
        const state = f.ledger(), action = state.pending;
        const frameOutput = resolve(f.scenario, 'E1-after'); mkdirSync(frameOutput);
        const frame = persistDirectFrame({ output: frameOutput, label: 'capture-001', pngName: 'F1.original.png',
            result, args, window: f.window, ownedPids: [101], provenance: { runID: f.runID, batch: 'E1',
                sourceSha256: sha(f.source), reservationID: action.reservationID, capturedAtMs } });
        assert.ok(frame.pngPath); await f.h.embedded.server.close();
        mkdirSync(output); outputCreated = true;
        const creationTime = new Date(state.providerClock.startMs).toISOString();
        const nodeExecutable = realpathSync.native(process.execPath);
        const child = { command: nodeExecutable, args: [resolve(f.root, 'tools/starsector-proxy/guarded-http-child.mjs')],
            cwd: '.', processOwnership: 'windows-job-object', status: 0, timedOut: false };
        const readyPath = resolve(output, 'guarded-http-ready.private.json');
        json(readyPath, { schemaVersion: 1, at: creationTime, runID: f.runID, batch: 'E1', sourceSha256: sha(f.source),
            startupTimeoutMs: 20000, consentMode: 'finite-explicit-consent',
            node: { pid: 39012, parentPid: 39011, creationTime, executable: nodeExecutable, sessionId: 1 },
            helper: { pid: 39013, parentPid: 39012, creationTime, executable: resolve(f.scenario, 'synthetic-provider-helper.exe'), sessionId: 1 },
            supervisorJobPids: [39012, 39013], query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' });
        const terminalPath = resolve(output, 'owned-provider-terminal.private.json');
        const terminal = { at: new Date().toISOString(), verifierPid: 39011, verifierStatus: 0, signal: null,
            deadline: new Date(state.providerClock.deadlineMs).toISOString(), closeFailed: false, overflow: false, spawnFailed: false,
            terminalObserved: true, cleanupUnknown: false, terminalBoundToChild: true, verdict: 'PASS',
            steps: [child], terminalReceipt: terminalPath }; json(terminalPath, terminal);
        json(resolve(output, 'owned-provider.stdout.private.txt'), { ok: true, command: 'verify', result: { verdict: 'PASS', steps: [child] } });
        writeFileSync(resolve(output, 'owned-provider.stderr.private.txt'), '');
        const receipt = { schemaVersion: 1, kind: 'STARSECTOR_E1_MENU_INSPECTION', scenario: f.scenario,
            flow: f.plan.campaign.flow, rootRecordKey: f.campaign.rootClock.recordKey, controllerSha256: sha(f.controllerPath),
            e1: { sourceSha256: sha(f.source), claimSha256: sha(resolve(f.scenario, 'guard-E1-consumed.private.json')),
                runID: f.runID, reservationID: action.reservationID },
            frame: { pngPath: frame.pngPath, pngSha256: sha(frame.pngPath), metadataPath: frame.metadataPath,
                metadataSha256: sha(frame.metadataPath), capturedAtMs },
            inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                state: 'ORDINARY_GAME_MENU', exitCoordinate: [40, 80] },
            provider: { readySha256: sha(readyPath), terminalSha256: sha(terminalPath), observedClosedAtMs: Date.now(), providerReaped: true } };
        const menuPath = resolve(f.scenario, 'E1-menu-inspected.private.json'); json(menuPath, receipt);
        const plan = { ...f.plan, batches: [{ id: 'E2' }], e1MenuReceipt: { path: menuPath, sha256: sha(menuPath) } };
        delete /** @type {any} */ (plan).b1MenuReceipt;
        const planPath = resolve(f.scenario, 'E2-plan.private.json'); json(planPath, plan);
        const lease = { ...f.lease, planSha256: sha(planPath), e1MenuReceipt: plan.e1MenuReceipt };
        delete /** @type {any} */ (lease).b1MenuReceipt;
        const leasePath = resolve(f.scenario, 'E2-lease.private.json'); json(leasePath, lease);
        const ownedReceiptPath = resolve(f.scenario, 'driver-E2-owned.private.json');
        json(ownedReceiptPath, JSON.parse(readFileSync(f.auth.ownedReceiptPath, 'utf8')));
        const source = resolve(f.scenario, 'E2-authorization.private.json');
        json(source, { ...f.auth, batch: 'E2', planPath, planSha256: sha(planPath), leasePath, leaseSha256: sha(leasePath),
            ownedReceiptPath, ownedReceiptSha256: sha(ownedReceiptPath) });
        return { ...f, source, runID: f.runID + '-exit', receipt, menuPath, plan, lease, terminal, terminalPath, close };
    } catch (error) { await close(); throw error; }
}
