// @ts-check
// Real guard/consent/vendor path with generated OS observations and closure only.
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync, rmSync, realpathSync } from 'node:fs';
import { basename, resolve, relative, isAbsolute } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { json, filePin as rawFilePin } from './seed-test-support.mjs';
import { claimSeededLauncher, seedInputSession, seedTarget } from './campaign-input-test-support.mjs';
import { persistDirectFrame } from '../direct-frames.mjs';

/** @param {string} path */
const filePin = path => { const pin = rawFilePin(path); return { ...pin, sha256: pin.sha256.toLowerCase() }; };

/** @param {{inspection?: boolean, generation?: boolean}} [options] */
export async function completedSeedLaunch(options = {}) {
    const f = campaignLaunchFixture(options);
    const schemaVersion = options.generation ? 5 : options.inspection ? 4 : 3;
    const helper = resolve(f.scenario, 'synthetic-provider-helper.exe');
    writeFileSync(helper, 'Generated identity only. Never executed.');
    const pinsPath = resolve(f.scenario, 'frozen-support-pins.private.json');
    const pins = JSON.parse(readFileSync(pinsPath, 'utf8'));
    pins.pins.push(...[helper, realpathSync.native(process.execPath)].map(path => ({ path, sha256: filePin(path).sha256 })));
    f.launchLease.supportPinsSha256 = json(pinsPath, pins).sha256;
    f.phaseLease.supportPinsSha256 = f.launchLease.supportPinsSha256;
    f.refreshLaunch();
    const originalLaunchBytes = [f.planPath, f.leasePath].map(path => readFileSync(path));
    const authority = claimSeededLauncher(f, 'synthetic-seed-handoff-' + basename(f.scenario));
    const outputParent = resolve(f.root, '.tmp/starsector-proxy');
    const output = resolve(outputParent, authority.runID);
    /** @type {string[]} */ const effects = [];
    let window = { ...seedTarget };
    const h = seedInputSession(authority, tool => effects.push(tool), { window: () => window });
    const ledger = () => JSON.parse(readFileSync(resolve(f.scenario, 'guard-launch-history.private.json'), 'utf8'));
    let outputCreated = false;
    async function close() {
        await h.embedded.server.close();
        if (outputCreated) {
            const suffix = relative(outputParent, output);
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix));
            rmSync(output, { recursive: true });
        }
        f.close();
    }
    try {
        await h.before();
        assert.notEqual((await h.call('left_click', { target_window_id: 19, target_app: 'java.exe',
            focus_strategy: 'strict', coordinate: [40, 80] })).isError, true);
        window = { ...seedTarget, windowId: 21 };
        await h.call('get_window', { window_id: 21 });
        const args = { target_window_id: 21, quality: 0, width: 480 };
        const result = await h.call('screenshot', args), capturedAtMs = Date.now();
        const state = ledger(), action = state.pending;
        const frameOutput = resolve(f.scenario, 'B1-after'); mkdirSync(frameOutput);
        const frame = persistDirectFrame({ output: frameOutput, label: 'capture-001', pngName: 'F1.original.png',
            result, args, window, ownedPids: [101], provenance: { runID: authority.runID, batch: 'B1',
                sourceSha256: filePin(authority.source).sha256, reservationID: action.reservationID, capturedAtMs } });
        assert.ok(frame.pngPath);
        await h.embedded.server.close();
        mkdirSync(output); outputCreated = true;
        const creationTime = new Date(state.providerClock.startMs).toISOString();
        const nodeExecutable = realpathSync.native(process.execPath);
        const child = { command: nodeExecutable, args: [resolve(f.root, 'tools/starsector-proxy/guarded-http-child.mjs')],
            cwd: '.', processOwnership: 'windows-job-object', status: 0, timedOut: false };
        const readyPath = resolve(output, 'guarded-http-ready.private.json');
        const ready = { schemaVersion: 1, at: creationTime, runID: authority.runID, batch: 'B1',
            sourceSha256: filePin(authority.source).sha256, startupTimeoutMs: 20000, consentMode: 'finite-explicit-consent',
            node: { pid: 39002, parentPid: 39001, creationTime, executable: nodeExecutable, sessionId: 1 },
            helper: { pid: 39003, parentPid: 39002, creationTime, executable: helper, sessionId: 1 },
            supervisorJobPids: [39002, 39003], query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' };
        json(readyPath, ready);
        const terminalPath = resolve(output, 'owned-provider-terminal.private.json');
        const terminal = { at: new Date().toISOString(), verifierPid: 39001, verifierStatus: 0, signal: null,
            deadline: new Date(state.providerClock.deadlineMs).toISOString(), closeFailed: false, overflow: false, spawnFailed: false,
            terminalObserved: true, cleanupUnknown: false, terminalBoundToChild: true, verdict: 'PASS',
            steps: [child], terminalReceipt: terminalPath };
        json(terminalPath, terminal);
        json(resolve(output, 'owned-provider.stdout.private.txt'), { ok: true, command: 'verify', result: { verdict: 'PASS', steps: [child] } });
        writeFileSync(resolve(output, 'owned-provider.stderr.private.txt'), '');
        const receipt = { schemaVersion: 1, kind: 'STARSECTOR_B1_MAIN_MENU_INSPECTION', scenario: f.scenario,
            flow: f.plan.campaignControl.profile, rootRecordKey: f.campaign.rootClock.recordKey,
            controllerSha256: filePin(f.controllerPath).sha256,
            b1: { sourceSha256: filePin(authority.source).sha256,
                claimSha256: filePin(resolve(f.scenario, 'guard-B1-consumed.private.json')).sha256,
                runID: authority.runID, reservationID: action.reservationID },
            frame: { pngPath: frame.pngPath, pngSha256: filePin(frame.pngPath).sha256, metadataPath: frame.metadataPath,
                metadataSha256: filePin(frame.metadataPath).sha256, capturedAtMs },
            inspection: { issuer: '/root', driver: '/root/recovery_game_driver', inspectedAtMs: Date.now(),
                state: 'STARSECTOR_MAIN_MENU', ...(options.inspection || options.generation ? { newGameCoordinate: [40, 80] } : { loadCoordinate: [40, 80] }) },
            provider: { readySha256: filePin(readyPath).sha256, terminalSha256: filePin(terminalPath).sha256,
                observedClosedAtMs: Date.now(), providerReaped: true } };
        const receiptPath = resolve(f.scenario, 'B1-menu-inspected.private.json'); json(receiptPath, receipt);
        const campaign = { ...f.campaign, flow: receipt.flow, windowId: 21 };
        const b1MenuReceipt = { path: receiptPath, sha256: filePin(receiptPath).sha256 };
        const plan = { ...f.phasePlan, campaign, b1MenuReceipt };
        json(f.phasePlanPath, plan);
        const lease = { ...f.phaseLease, schemaVersion, phase: campaign.flow, campaign, b1MenuReceipt,
            planSha256: filePin(f.phasePlanPath).sha256 };
        json(f.phaseLeasePath, lease);
        const auth = JSON.parse(readFileSync(f.source, 'utf8'));
        Object.assign(auth, { schemaVersion, planSha256: filePin(f.phasePlanPath).sha256,
            leaseSha256: filePin(f.phaseLeasePath).sha256 }); json(f.source, auth);
        return { ...f, authority, originalLaunchBytes, ledger, receipt, receiptPath, plan, lease, auth,
            ready, readyPath, terminal, terminalPath, effects, close };
    } catch (error) { await close(); throw error; }
}
