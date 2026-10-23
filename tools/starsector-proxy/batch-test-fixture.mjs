// @ts-check
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, basename, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { syntheticFixtureBinding } from './fixture-test-binding.mjs';
import { syntheticMenuLaunch } from './menu-launch-test-grant.mjs';
export const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
const base = resolve(root, '.tmp/starsector-proxy/scenarios');
/** @param {string} path @param {any} value */
export const write = (path, value) => writeFileSync(path, JSON.stringify(value) + '\n');
/** @param {string} path */
const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
export function fixture() {
    mkdirSync(base, { recursive: true });
    const scenario = mkdtempSync(resolve(base, 'synthetic-batch-authorization-'));
    const now = Date.now();
    const planPath = resolve(scenario, 'plan.json'), leasePath = resolve(scenario, 'native-lane-lease.private.json');
    const ownedReceiptPath = resolve(scenario, 'driver-B1-owned.private.json');
    const pinsPath = resolve(scenario, 'frozen-support-pins.private.json');
    const fixturePath = resolve(scenario, 'protected-fixture-receipt.private.json');
    const controllerPath = resolve(scenario, 'controller-start.private.json');
    write(planPath, { scenarioId: basename(scenario), paths: { scenario }, fixtureBinding: syntheticFixtureBinding(scenario), batches: [{ id: 'B1' }],
        bounds: { maxScenarioActionsIncludingActivations: 8, maxProxyInvocations: 3, proxyTimeoutMs: 180000, ownedGameHardTimeoutMs: 480000 } });
    write(pinsPath, { synthetic: true, scenario }); write(fixturePath, { synthetic: true });
    const lease = { schemaVersion: 1, authorized: true, phase: 'MENU_SMOKE_ONLY', issuer: '/root', driver: '/root/recovery_game_driver',
        planSha256: sha(planPath), ownedGameHardTimeoutMs: 480000, maxActions: 8, maxProxyInvocations: 3, campaignLoadSaveAllowed: false,
        supportPinsPath: pinsPath, supportPinsSha256: sha(pinsPath), fixtureReceiptSha256: sha(fixturePath) };
    const controller = { at: new Date(now - 2000).toISOString(), controllerPid: 100, sessionId: 1, supervisorJobPids: [100, 101],
        hardTimeoutMs: 480000, installation: resolve(scenario, 'installation'), fixtureReceiptSha256: sha(fixturePath), rootLeaseSha256: '' };
    write(controllerPath, controller); write(leasePath, lease);
    write(ownedReceiptPath, { processes: [{ pid: 101, inSupervisorJob: true, supervisorJobPids: [100, 101], sessionId: 1,
        creationTime: new Date(now - 1000).toISOString(), executable: resolve(scenario, 'installation/jre/bin/java.exe') }] });
    const menu = syntheticMenuLaunch(root, scenario); Object.assign(lease, menu.lease); Object.assign(controller, menu.controller);
    const auth = { schemaVersion: 1, scenario, batch: 'B1', planPath, planSha256: sha(planPath), leasePath,
        leaseSha256: sha(leasePath), ownedReceiptPath, ownedReceiptSha256: sha(ownedReceiptPath) };
    const path = resolve(scenario, 'B1-authorization.private.json'); write(path, auth);
    return { scenario, now, auth, path, lease, controller, leasePath, controllerPath,
        refresh() { write(leasePath, lease); controller.rootLeaseSha256 = sha(leasePath); write(controllerPath, controller);
            auth.leaseSha256 = sha(leasePath); write(path, auth); },
        close() {
            const suffix = relative(base, resolve(scenario));
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix));
            rmSync(scenario, { recursive: true, force: true });
        } };
}
