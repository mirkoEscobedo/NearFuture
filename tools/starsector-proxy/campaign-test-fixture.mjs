// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fixture, root, write } from './batch-test-fixture.mjs';
export { root };
/** @param {string} path */
export const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
export const campaignFlow = 'PAUSED_CAMPAIGN_EXIT_NO_SAVE';
export const target = { pid: 101, windowId: 19, title: 'Starsector', bundleId: 'java.exe', isFocused: true,
    isOnScreen: true, bounds: { x: 10, y: 10, width: 480, height: 220 } };
export const escapeArgs = { text: 'escape', repeat: 1, target_window_id: 19,
    target_app: 'java.exe', focus_strategy: 'strict' };
/** Synthetic root data composed from the existing owned-launch fixture. No game,
 * native addon, approval token or caller-clock origin is created by this helper. */
export function campaignFixture() {
    const f = fixture(), runID = 'campaign-escape-synthetic';
    const planPath = resolve(f.scenario, 'plan.json'), source = resolve(f.scenario, 'E1-authorization.private.json');
    const ownedReceiptPath = resolve(f.scenario, 'driver-E1-owned.private.json');
    write(ownedReceiptPath, JSON.parse(readFileSync(f.auth.ownedReceiptPath, 'utf8')));
    const startMs = f.now - 2000, sessionWatchdogMs = 7200000;
    const effectiveDeadlineMs = f.now + 600000;
    const deadlineUtc = new Date(effectiveDeadlineMs).toISOString().replace('Z', '0000Z');
    const rootClock = { recordKey: 'synthetic-campaign-root-call', startMs,
        deadlineMs: startMs + sessionWatchdogMs, sessionWatchdogMs, cleanupReserveMs: 30000,
        callerAdmissionDeadlineMs: effectiveDeadlineMs, originalCallerAdmissionDeadlineMs: effectiveDeadlineMs,
        producerRecordKey: 'synthetic-campaign-producer', effectiveDeadlineMs };
    const campaign = { flow: campaignFlow, windowId: target.windowId, rootClock,
        producerWitness: { producerRecordKey: rootClock.producerRecordKey, controllerPid: f.controller.controllerPid,
            controllerCreationTime: f.controller.at, controllerKernelCreationFileTime: '134371266963297182',
            conservativeOuterDeadlineUtc: deadlineUtc, controllerDeadlineUtc: deadlineUtc,
            effectiveDeadlineMs, sessionWatchdogMs },
        rootCallRecord: { key: rootClock.recordKey, value: { startMs,
            args: { cmd: 'SYNTHETIC_ROOT_NOT_RUN' }, raw: { session_id: 777 } } },
        historyBaseline: { overallAttempts: 33, uiAttempts: 31, reservedActions: 56, sentActions: 54 },
        maxReservedActions: 64 };
    const plan = JSON.parse(readFileSync(planPath, 'utf8'));
    Object.assign(plan, { campaign, batches: [{ id: 'E1' }, { id: 'E2' }],
        directHostConsent: { schemaVersion: 1, mode: 'finite-explicit-consent', maxCallsPerBatch: 32 } });
    plan.bounds.ownedGameHardTimeoutMs = sessionWatchdogMs;
    write(planPath, plan);
    Object.assign(f.controller, { hardTimeoutMs: sessionWatchdogMs });
    write(f.controllerPath, f.controller);
    Object.assign(f.lease, { schemaVersion: 2, phase: campaignFlow, campaign,
        ownedGameHardTimeoutMs: sessionWatchdogMs, planSha256: sha(planPath), directHostConsent: plan.directHostConsent });
    write(f.leasePath, f.lease);
    const auth = { ...f.auth, schemaVersion: 2, batch: 'E1', planSha256: sha(planPath),
        leaseSha256: sha(f.leasePath), ownedReceiptPath, ownedReceiptSha256: sha(ownedReceiptPath) };
    write(source, auth);
    const historyPath = resolve(f.scenario, 'guard-launch-history.private.json');
    write(historyPath, { schemaVersion: 2, kind: 'STARSECTOR_FINITE_LAUNCH_HISTORY',
        rootRecordKey: rootClock.recordKey, controllerSha256: sha(f.controllerPath),
        history: { ...campaign.historyBaseline }, pending: null, uncertain: false });
    return { ...f, root, source, runID, auth, plan, campaign, historyPath,
        history: () => JSON.parse(readFileSync(historyPath, 'utf8')) };
}
