// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync, lstatSync, realpathSync } from 'node:fs';
import { resolve, dirname, relative, isAbsolute } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import {campaignProviderClock} from './seed-provider-budget.mjs';

/** @param {any} condition @param {string} message @returns {asserts condition} */
export function requireThat(condition, message) { if (!condition) throw new Error('Campaign authority refused: ' + message); }
/** @param {string} path @param {number} [limit] */
export function file(path, limit = 256 * 1024) {
    for (let part = resolve(path);; part = dirname(part)) {
        requireThat(!lstatSync(part).isSymbolicLink(), 'ordinary custody required');
        if (dirname(part) === part) break;
    }
    const stat = lstatSync(path);
    requireThat(stat.isFile() && stat.size <= limit, 'bounded ordinary custody required');
    const bytes = readFileSync(path);
    return { bytes, sha256: createHash('sha256').update(bytes).digest('hex') };
}
/** @param {string} path @param {number} [limit] */
export function read(path, limit = 256 * 1024) {
    const found = file(path, limit);
    return { ...found, value: JSON.parse(found.bytes.toString('utf8').replace(/^\uFEFF/, '')) };
}
/** @param {any} value @param {string[]} fields */
export function closed(value, fields) {
    return value && typeof value === 'object' && !Array.isArray(value) &&
        Object.keys(value).sort().join() === [...fields].sort().join();
}
/** @param {string} parent @param {string} child */
export function inside(parent, child) {
    const suffix = relative(resolve(parent), resolve(child));
    return !!suffix && !suffix.startsWith('..') && !isAbsolute(suffix);
}
/** The same known-closed provider proof is shared by the two fixed transfers.
 * This neither changes ownership nor supplies a new deadline. @param {any} context */
export function knownProviderClosure(context) {
    return ownedProviderClosure(context,['B1','E1']);
}
/** Terminal-only schema6 E2 evidence, never a transfer or fresh authority.
 * @param {any} context */
export function knownSeedTerminalClosure(context) {
    requireThat((context.binding?.schemaVersion===6||context.binding?.schemaVersion===7&&
        context.binding.campaign?.flow==='DISPOSABLE_SEED_CREATION_INITIAL_FOCUS')&&context.binding.batch==='E2'&&context.initial===context.binding,
        'terminal seed provider evidence required');
    return ownedProviderClosure(context,['E2']);
}
/** @param {any} context @param {string[]} allowedBatches */
function ownedProviderClosure(context,allowedBatches) {
    const { binding, initial, initialSourceSha256, proof, providerClock, action, now } = context;
    const { frame, inspection } = proof;
    requireThat(allowedBatches.includes(initial.batch), 'known provider batch required');
    const provider = proof.provider, output = resolve(binding.workspaceRoot, '.tmp/starsector-proxy', initial.runID);
    const ready = read(resolve(output, 'guarded-http-ready.private.json'));
    const terminalPath = resolve(output, 'owned-provider-terminal.private.json'), terminal = read(terminalPath), t = terminal.value, r = ready.value;
    const spool = read(resolve(output, 'owned-provider.stdout.private.txt'), 2 * 1024 * 1024).value;
    const nodeExecutable = realpathSync.native(process.execPath);
    const expectedStep = { command: nodeExecutable, args: [resolve(binding.workspaceRoot, 'tools/starsector-proxy/guarded-http-child.mjs')],
        cwd: '.', processOwnership: 'windows-job-object', status: 0, timedOut: false };
    /** @param {any} step */
    const criticalStep = step => ({ command: step?.command, args: step?.args, cwd: step?.cwd,
        processOwnership: step?.processOwnership, status: step?.status, timedOut: step?.timedOut });
    const pins = read(resolve(binding.scenario, 'frozen-support-pins.private.json')).value.pins;
    const helperPin = pins?.find((/** @type {any} */ pin) => typeof pin.path === 'string' && resolve(pin.path) === resolve(r.helper?.executable ?? ''));
    requireThat(closed(provider, ['readySha256', 'terminalSha256', 'observedClosedAtMs', 'providerReaped']) &&
        provider.readySha256 === ready.sha256 && provider.terminalSha256 === terminal.sha256 && provider.providerReaped === true &&
        t.terminalObserved === true && t.cleanupUnknown === false && t.closeFailed === false && t.overflow === false && t.spawnFailed === false &&
        t.terminalBoundToChild === true && t.verifierStatus === 0 && t.signal === null && t.verdict === 'PASS' &&
        t.terminalReceipt === terminalPath && t.steps?.length === 1 && isDeepStrictEqual(criticalStep(t.steps[0]), expectedStep) &&
        spool.ok === true && spool.command === 'verify' && spool.result?.verdict === 'PASS' && spool.result.steps?.length === 1 &&
        isDeepStrictEqual(criticalStep(spool.result.steps[0]), expectedStep) &&
        file(resolve(output, 'owned-provider.stderr.private.txt'), 2 * 1024 * 1024).bytes.length === 0 &&
        r.schemaVersion === 1 && r.runID === initial.runID && r.batch === initial.batch && r.sourceSha256 === initialSourceSha256 &&
        r.startupTimeoutMs === 20000 && r.consentMode === 'finite-explicit-consent' &&
        Number.isSafeInteger(t.verifierPid) && t.verifierPid > 0 && r.node?.parentPid === t.verifierPid &&
        r.node.executable === nodeExecutable && r.node.sessionId === read(resolve(binding.scenario, 'controller-start.private.json')).value.sessionId &&
        Number.isSafeInteger(r.node.pid) && Number.isSafeInteger(r.helper?.pid) && r.node.pid > 0 && r.helper.pid > 0 && r.node.pid !== r.helper.pid &&
        r.helper.parentPid === r.node.pid && r.helper.sessionId === r.node.sessionId &&
        r.supervisorJobPids?.includes(r.node.pid) && r.supervisorJobPids.includes(r.helper.pid) &&
        r.query === 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' && helperPin &&
        helperPin.sha256?.toLowerCase() === file(r.helper.executable, 32 * 1024 * 1024).sha256,
    'known provider closure required');
    const safe = Number.isSafeInteger, closedAt = Date.parse(t.at), actualDeadline = Date.parse(t.deadline);
    requireThat(safe(providerClock?.startMs) && providerClock.deadlineMs === campaignProviderClock(initial,providerClock.startMs).deadlineMs && safe(action.sentAtMs) && safe(action.knownResultAtMs) &&
        safe(frame.capturedAtMs) && safe(inspection.inspectedAtMs) && safe(provider.observedClosedAtMs) &&
        action.sentAtMs >= providerClock.startMs && action.knownResultAtMs >= action.sentAtMs &&
        frame.capturedAtMs >= action.knownResultAtMs && closedAt >= frame.capturedAtMs && closedAt <= actualDeadline &&
        actualDeadline <= providerClock.deadlineMs && provider.observedClosedAtMs >= closedAt && provider.observedClosedAtMs <= now &&
        inspection.inspectedAtMs >= frame.capturedAtMs && inspection.inspectedAtMs <= now &&
        Date.parse(r.at) >= providerClock.startMs && Date.parse(r.at) <= action.sentAtMs &&
        Date.parse(r.node.creationTime) >= providerClock.startMs && Date.parse(r.node.creationTime) <= closedAt &&
        Date.parse(r.helper.creationTime) >= Date.parse(r.node.creationTime) && Date.parse(r.helper.creationTime) <= closedAt,
    'known provider closure chronology required');
    return closedAt;
}
