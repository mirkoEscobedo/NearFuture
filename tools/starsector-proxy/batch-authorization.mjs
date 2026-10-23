// @ts-check
import { readFileSync, lstatSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { resolve, dirname, basename, relative, isAbsolute } from 'node:path';
import { validateBatchBinding } from './batch-session.mjs';
import { readCampaignAuthorization, claimCampaignOwner } from './campaign-authority.mjs';
import { readMenuLaunchGrant } from './menu-launch-grant.mjs';

/** @param {string} parent @param {string} child */
function inside(parent, child) {
    const path = relative(parent, child);
    return path !== '' && !path.startsWith('..') && !isAbsolute(path);
}
/** @param {string} path */
function ordinary(path) {
    let part = resolve(path);
    while (true) {
        if (lstatSync(part).isSymbolicLink()) throw new Error('Batch paths must be ordinary, without reparse links');
        const parent = dirname(part); if (parent === part) break; part = parent;
    }
}
/** @param {string} path */
function read(path) {
    ordinary(path);
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.size > 256 * 1024) throw new Error('Bounded ordinary batch file required');
    const bytes = readFileSync(path);
    return { value: JSON.parse(bytes.toString('utf8').replace(/^\uFEFF/, '')),
        sha256: createHash('sha256').update(bytes).digest('hex') };
}
/** Read-only validation before invoking any client/server or creating a claim.
 * @param {string} path @param {{root: string, runID: string, now?: number}} options */
export function readBatchAuthorization(path, { root, runID, now = Date.now() }) {
    const sourcePath = resolve(path);
    const source = read(sourcePath);
    const auth = source.value;
    if ([2, 3].includes(auth?.schemaVersion)) return readCampaignAuthorization(sourcePath, { root, runID, now });
    const keys = ['batch', 'leasePath', 'leaseSha256', 'ownedReceiptPath', 'ownedReceiptSha256',
        'planPath', 'planSha256', 'scenario', 'schemaVersion'];
    if (!auth || Array.isArray(auth) || Object.keys(auth).sort().join() !== keys.join() ||
        auth.schemaVersion !== 1 || typeof auth.scenario !== 'string' || !isAbsolute(auth.scenario)) {
        throw new Error('Closed caller batch authorization required');
    }
    const scenario = resolve(auth.scenario);
    if (!inside(resolve(root, '.tmp/starsector-proxy/scenarios'), scenario) || dirname(sourcePath) !== scenario ||
        typeof auth.batch !== 'string' || !['B1', 'B2', 'B3'].includes(auth.batch)) throw new Error('Unknown scenario or batch');
    const expectedPaths = { planPath: resolve(scenario, 'plan.json'),
        leasePath: resolve(scenario, 'native-lane-lease.private.json'),
        ownedReceiptPath: resolve(scenario, 'driver-' + auth.batch + '-owned.private.json') };
    /** @type {Record<string, any>} */ const files = {};
    for (const [key, expected] of Object.entries(expectedPaths)) {
        if (typeof auth[key] !== 'string' || resolve(auth[key]) !== expected ||
            typeof auth[key.replace('Path', 'Sha256')] !== 'string') throw new Error('Batch custody path mismatch');
        files[key] = read(expected);
        if (files[key].sha256 !== auth[key.replace('Path', 'Sha256')].toLowerCase()) throw new Error('Batch custody hash mismatch');
    }
    const plan = files.planPath.value, lease = files.leasePath.value, owned = files.ownedReceiptPath.value;
    const launchGrant = readMenuLaunchGrant({ root, scenario, plan, lease, planSha256: files.planPath.sha256 });
    if (plan.scenarioId !== basename(scenario) || resolve(plan.paths?.scenario ?? '') !== scenario ||
        plan.bounds?.maxScenarioActionsIncludingActivations !== 8 || plan.bounds?.maxProxyInvocations !== 3 ||
        plan.bounds?.proxyTimeoutMs !== 180000 || plan.bounds?.ownedGameHardTimeoutMs !== 480000 ||
        !plan.batches?.some(/** @param {any} batch */ batch => batch.id === auth.batch)) throw new Error('Unqualified finite menu plan');
    if (lease.authorized !== true || lease.schemaVersion !== 1 || lease.phase !== 'MENU_SMOKE_ONLY' ||
        lease.issuer !== '/root' || lease.driver !== '/root/recovery_game_driver' ||
        lease.planSha256?.toLowerCase() !== files.planPath.sha256 || lease.ownedGameHardTimeoutMs !== 480000 ||
        lease.maxActions !== 8 || lease.maxProxyInvocations !== 3 || lease.campaignLoadSaveAllowed !== false ||
        resolve(lease.supportPinsPath ?? '') !== resolve(scenario, 'frozen-support-pins.private.json')) throw new Error('Current finite root lease required');
    const pins = read(lease.supportPinsPath);
    if (pins.sha256 !== lease.supportPinsSha256?.toLowerCase()) throw new Error('Support manifest drift');
    if (typeof lease.fixtureReceiptSha256 !== 'string' || !/^[a-fA-F0-9]{64}$/.test(lease.fixtureReceiptSha256) ||
        read(resolve(scenario, 'protected-fixture-receipt.private.json')).sha256 !== lease.fixtureReceiptSha256.toLowerCase()) {
        throw new Error('Current root-admitted fixture binding required');
    }
    const controller = read(resolve(scenario, 'controller-start.private.json')).value;
    if (controller.rootLeaseSha256?.toLowerCase() !== files.leasePath.sha256 ||
        !isDeepStrictEqual(controller.launchGrant, launchGrant)) throw new Error('Original menu launch authority changed');
    const started = Date.parse(controller.at);
    if (!Number.isFinite(started) || started > now || now - started >= 480000 || controller.hardTimeoutMs !== 480000 ||
        !Number.isSafeInteger(controller.controllerPid) || controller.controllerPid <= 0 ||
        !Number.isSafeInteger(controller.sessionId) || controller.sessionId <= 0 ||
        !controller.supervisorJobPids?.includes(controller.controllerPid) ||
        resolve(controller.installation ?? '') !== resolve(scenario, 'installation') ||
        controller.fixtureReceiptSha256?.toLowerCase() !== lease.fixtureReceiptSha256?.toLowerCase() ||
        !Array.isArray(owned.processes) || !owned.processes.length) throw new Error('Current owned launch interval required');
    const installation = resolve(scenario, 'installation');
    const ownedPids = owned.processes.map(/** @param {any} process */ process => {
        if (!Number.isSafeInteger(process.pid) || process.pid <= 0 || process.inSupervisorJob !== true ||
            !process.supervisorJobPids?.includes(process.pid) || process.sessionId !== controller.sessionId ||
            typeof process.executable !== 'string' || !inside(installation, resolve(process.executable)) ||
            !Number.isFinite(Date.parse(process.creationTime)) || Date.parse(process.creationTime) < started ||
            Date.parse(process.creationTime) > now) throw new Error('Owned target receipt mismatch');
        return process.pid;
    });
    const binding = { schemaVersion: 1, batch: auth.batch, runID, scenario,
        planSha256: files.planPath.sha256, ownedPids };
    validateBatchBinding(binding);
    return { sourcePath, sourceSha256: source.sha256, binding,
        claimPath: resolve(scenario, 'guard-' + auth.batch + '-consumed.private.json') };
}

/** One successful claim per approved scenario batch, never reusable by another run.
 * @param {ReturnType<typeof readBatchAuthorization>} authorization */
export function claimBatchAuthorization(authorization) {
    writeFileSync(authorization.claimPath, JSON.stringify({ sourcePath: authorization.sourcePath,
        sourceSha256: authorization.sourceSha256, binding: authorization.binding }) + '\n', { flag: 'wx' });
    if ([2, 3].includes(authorization.binding.schemaVersion)) claimCampaignOwner(authorization.binding);
}

/** Independently re-read file/lease custody inside the owned MCP child before native initialization.
 * @param {string} path @param {{root: string, runID: string, now?: number}} options */
export function readClaimedBatchAuthorization(path, options) {
    const authorization = readBatchAuthorization(path, options);
    const claim = read(authorization.claimPath).value;
    if (claim.sourcePath !== authorization.sourcePath || claim.sourceSha256 !== authorization.sourceSha256 ||
        JSON.stringify(claim.binding) !== JSON.stringify(authorization.binding)) throw new Error('Batch claim belongs to another run or source');
    return authorization;
}
