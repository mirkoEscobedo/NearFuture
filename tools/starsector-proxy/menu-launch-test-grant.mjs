// @ts-check
// Filesystem test data only. This never represents a real launch or input grant.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { menuLaunchSources } from './menu-launch-grant.mjs';
const read = (/** @type {string} */ path) => JSON.parse(readFileSync(path, 'utf8'));
const write = (/** @type {string} */ path, /** @type {any} */ value) => writeFileSync(path, JSON.stringify(value) + '\n');
const sha = (/** @type {string} */ path) => createHash('sha256').update(readFileSync(path)).digest('hex');

/** Bind existing generated fixtures before their simulated launch.
 * @param {string} root @param {string} scenario */
export function syntheticMenuLaunch(root, scenario) {
    const planPath = resolve(scenario, 'plan.json'), leasePath = resolve(scenario, 'native-lane-lease.private.json');
    const controllerPath = resolve(scenario, 'controller-start.private.json');
    const plan = read(planPath), lease = read(leasePath), controller = read(controllerPath);
    const pinsPath = resolve(scenario, 'frozen-support-pins.private.json');
    const pins = read(pinsPath).pins ?? [];
    for (const name of menuLaunchSources) {
        const path = resolve(root, name);
        if (!pins.some((/** @type {any} */ pin) => pin.path === path)) pins.push({ path, sha256: sha(path) });
    }
    write(pinsPath, { pins });
    lease.supportPinsSha256 = sha(pinsPath);
    const grantPath = resolve(scenario, 'menu-launch-grant.private.json');
    write(grantPath, { schemaVersion: 1, kind: 'FINITE_MENU_LAUNCH_GRANT_V1', phase: 'CAMPAIGN_BINDING_ONLY',
        authorized: true, issuer: '/root', driver: '/root/recovery_game_driver', scenarioId: basename(scenario),
        fixtureBinding: plan.fixtureBinding, fixtureReceiptSha256: lease.fixtureReceiptSha256,
        supportPinsSha256: lease.supportPinsSha256, ownedGameHardTimeoutMs: 480000, campaignLoadSaveAllowed: false });
    plan.launchGrant = lease.launchGrant = { path: grantPath, sha256: sha(grantPath) };
    plan.ownership = { driver: '/root/recovery_game_driver', onlyGameInputDriver: true };
    lease.scenarioId = basename(scenario);
    write(planPath, plan); lease.planSha256 = sha(planPath); write(leasePath, lease);
    controller.launchGrant = lease.launchGrant; controller.rootLeaseSha256 = sha(leasePath);
    write(controllerPath, controller);
    return { lease, controller };
}
