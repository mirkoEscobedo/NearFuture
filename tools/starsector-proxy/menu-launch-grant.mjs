// @ts-check
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, relative, resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { fixtureProbeSources } from './fixture-binding.mjs';

export const menuLaunchSources = Object.freeze(['tools/starsector-proxy/menu-launch-grant.mjs',
    'tools/starsector-proxy/batch-authorization.mjs', 'tools/starsector-proxy/fixture-binding.mjs', ...fixtureProbeSources]);
const denied = ['seed', 'ownedNodeKill', 'recordingProfile', 'recordingProfileSha256', 'campaign', 'e1MenuReceipt'];
/** @param {boolean} valid @param {string} reason */
function requireThat(valid, reason) { if (!valid) throw new Error('Menu launch grant: ' + reason); }
/** @param {any} value @param {string[]} names */
function closed(value, names) {
    return value !== null && typeof value === 'object' && !Array.isArray(value)
        && Object.keys(value).sort().join() === [...names].sort().join();
}
/** @param {string} path @param {number} limit */
function bytes(path, limit) {
    for (let part = resolve(path);; part = dirname(part)) {
        requireThat(!lstatSync(part).isSymbolicLink(), 'ordinary ancestry required');
        if (part === dirname(part)) break;
    }
    const stat = lstatSync(path);
    requireThat(stat.isFile() && stat.size >= 2 && stat.size <= limit, 'bounded file required');
    const value = readFileSync(path);
    return { bytes: value, sha256: createHash('sha256').update(value).digest('hex') };
}
/** @param {any} descriptor @param {string} expected @param {string} label */
function descriptorFile(descriptor, expected, label) {
    requireThat(closed(descriptor, ['path', 'sha256']) && typeof descriptor.path === 'string'
        && isAbsolute(descriptor.path) && resolve(descriptor.path) === expected
        && typeof descriptor.sha256 === 'string' && /^[a-f0-9]{64}$/i.test(descriptor.sha256), 'exact ' + label + ' descriptor required');
    const file = bytes(expected, 16384);
    requireThat(file.sha256 === descriptor.sha256.toLowerCase(), label + ' descriptor digest changed');
    return JSON.parse(file.bytes.toString('utf8').replace(/^\uFEFF/, ''));
}

/** Frozen V1 launch authority; actual process/focus/image checks remain in their original boundaries.
 * @param {{root:string, scenario:string, plan:any, lease:any, planSha256:string}} input */
export function readMenuLaunchGrant({ root, scenario, plan, lease, planSha256 }) {
    requireThat(plan.scenarioId === basename(scenario) && resolve(plan.paths?.scenario ?? '') === scenario
        && plan.ownership?.driver === '/root/recovery_game_driver' && plan.ownership.onlyGameInputDriver === true
        && plan.bounds?.maxScenarioActionsIncludingActivations === 8 && plan.bounds.maxProxyInvocations === 3
        && plan.bounds.proxyTimeoutMs === 180000 && plan.bounds.ownedGameHardTimeoutMs === 480000
        && lease.schemaVersion === 1 && lease.scenarioId === basename(scenario) && lease.authorized === true
        && lease.phase === 'MENU_SMOKE_ONLY' && lease.issuer === '/root' && lease.driver === '/root/recovery_game_driver'
        && lease.planSha256?.toLowerCase() === planSha256 && lease.ownedGameHardTimeoutMs === 480000
        && lease.maxActions === 8 && lease.maxProxyInvocations === 3 && lease.campaignLoadSaveAllowed === false,
    'complete V1 menu scope required');
    requireThat(Array.isArray(plan.batches) && plan.batches.length >= 1 && plan.batches.length <= 3
        && plan.batches.some((/** @type {any} */ batch) => batch?.id === 'B1')
        && plan.batches.every((/** @type {any} */ batch) => ['B1', 'B2', 'B3'].includes(batch?.id))
        && new Set(plan.batches.map((/** @type {any} */ batch) => batch.id)).size === plan.batches.length,
    'closed launcher/menu batch scope required');
    for (const value of [plan, lease]) {
        requireThat(!denied.some(key => Object.hasOwn(value, key)), 'campaign extensions refused');
        requireThat(!Object.hasOwn(value, 'sessionWatchdogMs') || value.sessionWatchdogMs === 480000, 'fixed watchdog required');
    }
    requireThat(isDeepStrictEqual(plan.launchGrant, lease.launchGrant), 'plan and lease grant differ');
    const grant = descriptorFile(plan.launchGrant, resolve(scenario, 'menu-launch-grant.private.json'), 'launch grant');
    requireThat(closed(grant, ['schemaVersion', 'kind', 'phase', 'authorized', 'issuer', 'driver', 'scenarioId',
        'fixtureBinding', 'fixtureReceiptSha256', 'supportPinsSha256', 'ownedGameHardTimeoutMs', 'campaignLoadSaveAllowed'])
        && grant.schemaVersion === 1 && grant.kind === 'FINITE_MENU_LAUNCH_GRANT_V1'
        && grant.phase === 'CAMPAIGN_BINDING_ONLY' && grant.authorized === true && grant.issuer === '/root'
        && grant.driver === lease.driver && grant.scenarioId === basename(scenario)
        && grant.ownedGameHardTimeoutMs === 480000 && grant.campaignLoadSaveAllowed === false,
    'closed launch-only grant required');
    requireThat(isDeepStrictEqual(plan.fixtureBinding, grant.fixtureBinding), 'fixture binding descriptor differs');
    const binding = descriptorFile(grant.fixtureBinding, resolve(scenario, 'fixture-binding.private.json'), 'fixture binding');
    requireThat(closed(binding, ['schemaVersion', 'fixtureRoot', 'sourceInstallation', 'qualifiedRuntimeRoot',
        'candidateHead', 'runtimeManifestSha256', 'presentationAgentPath', 'presentationAgentSha256',
        'presentationClassSha256', 'driverRole']) && binding.schemaVersion === 1
        && binding.fixtureRoot === scenario && binding.driverRole === lease.driver, 'legacy menu fixture required');
    const receipt = bytes(resolve(scenario, 'protected-fixture-receipt.private.json'), 262144);
    requireThat(typeof lease.fixtureReceiptSha256 === 'string' && lease.fixtureReceiptSha256.toLowerCase() === receipt.sha256
        && typeof grant.fixtureReceiptSha256 === 'string' && grant.fixtureReceiptSha256.toLowerCase() === receipt.sha256,
    'fixture receipt changed');
    const pinsPath = resolve(scenario, 'frozen-support-pins.private.json'), pinsFile = bytes(pinsPath, 262144);
    requireThat(typeof lease.supportPinsPath === 'string' && isAbsolute(lease.supportPinsPath)
        && resolve(lease.supportPinsPath) === pinsPath && lease.supportPinsSha256?.toLowerCase() === pinsFile.sha256
        && grant.supportPinsSha256?.toLowerCase() === pinsFile.sha256, 'support manifest changed');
    const pins = JSON.parse(pinsFile.bytes.toString('utf8').replace(/^\uFEFF/, ''));
    requireThat(closed(pins, ['pins']) && Array.isArray(pins.pins) && pins.pins.length <= 1024, 'closed support manifest required');
    const seen = new Map();
    for (const pin of pins.pins) {
        requireThat(closed(pin, ['path', 'sha256']) && typeof pin.path === 'string' && isAbsolute(pin.path)
            && typeof pin.sha256 === 'string' && /^[a-f0-9]{64}$/i.test(pin.sha256), 'closed source pin required');
        const path = resolve(pin.path), suffix = relative(root, path);
        requireThat(suffix !== '' && !['.tmp', '.git'].includes(suffix.split(/[\\/]/)[0]) && !seen.has(path),
            'private authority or duplicate source pin refused');
        seen.set(path, pin.sha256.toLowerCase());
    }
    // The provider retains full installed-runtime/executable pin validation.
    // These are the maintained validators that must also be fresh at launch and each input.
    for (const source of menuLaunchSources) {
        const path = resolve(root, source);
        requireThat(seen.has(path), 'current launch tooling pins required');
        requireThat(bytes(path, 1048576).sha256 === seen.get(path), 'source custody required: source bytes changed');
    }
    return plan.launchGrant;
}
