// @ts-check
// Generated boundary files only: no game, provider, launcher or native input.
import { readFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { seedFixture, json, filePin, powershell, quote, repository } from './seed-test-support.mjs';
import { menuLaunchSources } from '../menu-launch-grant.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';

export function menuLaunchFixture() {
    const f = seedFixture(), scenario = f.target;
    const binding = JSON.parse(readFileSync(f.bindingPath, 'utf8'));
    binding.driverRole = '/root/recovery_game_driver';
    const bindingPin = json(f.bindingPath, binding);
    const fixtureBinding = { path: f.bindingPath, sha256: bindingPin.sha256 };
    const supportPinsPath = resolve(scenario, 'frozen-support-pins.private.json');
    json(supportPinsPath, { pins: menuLaunchSources
        .map(name => ({ path: resolve(repository, name), sha256: filePin(resolve(repository, name)).sha256 })) });
    const grantPath = resolve(scenario, 'menu-launch-grant.private.json');
    const grant = { schemaVersion: 1, kind: 'FINITE_MENU_LAUNCH_GRANT_V1', phase: 'CAMPAIGN_BINDING_ONLY',
        authorized: true, issuer: '/root', driver: binding.driverRole, scenarioId: basename(scenario),
        fixtureBinding, fixtureReceiptSha256: filePin(f.frozenPath).sha256,
        supportPinsSha256: filePin(supportPinsPath).sha256, ownedGameHardTimeoutMs: 480000,
        campaignLoadSaveAllowed: false };
    const launchGrant = { path: grantPath, sha256: json(grantPath, grant).sha256 };
    const planPath = resolve(scenario, 'plan.json'), leasePath = resolve(scenario, 'native-lane-lease.private.json');
    const plan = { scenarioId: basename(scenario), paths: { scenario }, fixtureBinding, launchGrant,
        ownership: { driver: binding.driverRole, onlyGameInputDriver: true }, batches: [{ id: 'B1' }],
        bounds: { maxScenarioActionsIncludingActivations: 8, maxProxyInvocations: 3,
            proxyTimeoutMs: 180000, ownedGameHardTimeoutMs: 480000 } };
    const lease = { schemaVersion: 1, scenarioId: basename(scenario), authorized: true, phase: 'MENU_SMOKE_ONLY',
        issuer: '/root', driver: binding.driverRole, launchGrant, planSha256: json(planPath, plan).sha256,
        ownedGameHardTimeoutMs: 480000, maxActions: 8, maxProxyInvocations: 3, campaignLoadSaveAllowed: false,
        supportPinsPath, supportPinsSha256: filePin(supportPinsPath).sha256,
        fixtureReceiptSha256: filePin(f.frozenPath).sha256 };
    json(leasePath, lease);
    const now = Date.now(), controllerPath = resolve(scenario, 'controller-start.private.json');
    const controller = { at: new Date(now - 2000).toISOString(), controllerPid: 100, sessionId: 1,
        supervisorJobPids: [100, 101], hardTimeoutMs: 480000, installation: f.installation,
        fixtureReceiptSha256: lease.fixtureReceiptSha256, launchGrant,
        rootLeaseSha256: filePin(leasePath).sha256 };
    json(controllerPath, controller);
    const ownedReceiptPath = resolve(scenario, 'driver-B1-owned.private.json');
    json(ownedReceiptPath, { processes: [{ pid: 101, inSupervisorJob: true, supervisorJobPids: [100, 101],
        sessionId: 1, creationTime: new Date(now - 1000).toISOString(),
        executable: resolve(f.installation, 'jre/bin/java.exe') }] });
    const path = resolve(scenario, 'B1-authorization.private.json');
    const auth = { schemaVersion: 1, scenario, batch: 'B1', planPath, planSha256: filePin(planPath).sha256,
        leasePath, leaseSha256: filePin(leasePath).sha256, ownedReceiptPath,
        ownedReceiptSha256: filePin(ownedReceiptPath).sha256 };
    json(path, auth);
    function refreshAuthority() {
        auth.planSha256 = lease.planSha256 = json(planPath, plan).sha256;
        auth.leaseSha256 = controller.rootLeaseSha256 = json(leasePath, lease).sha256;
        controller.launchGrant = lease.launchGrant; json(controllerPath, controller); json(path, auth);
    }
    function refreshGrant() {
        grant.supportPinsSha256 = lease.supportPinsSha256 = filePin(supportPinsPath).sha256;
        plan.launchGrant = lease.launchGrant = { path: grantPath, sha256: json(grantPath, grant).sha256 };
        refreshAuthority();
    }
    return { ...f, scenario, now, grant, grantPath, plan, planPath, lease, leasePath, controller, controllerPath, path,
        supportPinsPath, refreshAuthority, refreshGrant,
        admit() { return powershell(['-Command', '& ' + quote(resolve(repository, 'tools/starsector-proxy/fixture/protection-check.ps1'))
            + ' -BindingPath ' + quote(f.bindingPath) + ' -BindingSha256 ' + quote(bindingPin.sha256)
            + ' -Phase admit-launch -EvidenceRoot ' + quote(scenario) + ' | ConvertTo-Json -Depth 6 -Compress']); },
        input() { return readBatchAuthorization(path, { root: repository, runID: 'composed-menu', now }); } };
}
