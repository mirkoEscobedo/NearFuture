// @ts-check
// Generated files only. These are not game, save compatibility or native evidence.
import assert from 'node:assert/strict';
import { basename, resolve } from 'node:path';
import { unlinkSync, readFileSync } from 'node:fs';
import { seedFixture, json, filePin, powershell, repository, quote } from './seed-test-support.mjs';
import { campaignLaunchSources } from '../campaign-authority.mjs';

/** @param {{inspection?: boolean, generation?: boolean, historyBaseline?: {overallAttempts: number, uiAttempts: number, reservedActions: number, sentActions: number}}} [options] */
export function campaignLaunchFixture(options = {}) {
    const f = seedFixture({ driverRole: '/root/recovery_game_driver' }), scenario = f.target;
    const noSeed = options.inspection || options.generation;
    if (options.generation) {
        const binding = JSON.parse(readFileSync(f.bindingPath, 'utf8'));
        Object.assign(binding, {schemaVersion: 3, fixtureProfile: 'VANILLA_UNCOMPRESSED_GENERATION'});
        Object.assign(f.bindingPin, json(f.bindingPath, binding));
        const frozen = JSON.parse(readFileSync(f.frozenPath, 'utf8'));
        frozen.settingsSha256 = json(resolve(f.installation, 'starsector-core/data/config/settings.json'),
            {autosaveFeatureEnabled: false, autosaveEnabledByDefault: false, compressSaveGameData: false}).sha256;
        frozen.enabledModsSha256 = json(resolve(f.installation, 'mods/enabled_mods.json'), {enabledMods: []}).sha256;
        json(f.frozenPath, frozen);
    }
    if (noSeed) {
        for (const name of ['seed-manifest.private.json', 'seed-stage-authority.private.json', 'seed-stage-plan.private.json']) {
            unlinkSync(resolve(scenario, name));
        }
    } else assert.equal(f.stage().status, 0, 'generated disposable seed staging');
    const startMs = Date.now() - 2000, deadlineMs = startMs + 480000;
    const callerAdmissionDeadlineMs = startMs + 420000;
    const planPath = resolve(scenario, 'plan.json');
    const leasePath = resolve(scenario, 'native-lane-lease.private.json');
    const seed = noSeed ? undefined : { mode: 'LOAD_ONE_PRIVATE_DISPOSABLE_SEED',
        manifestSha256: filePin(f.manifestPath).sha256,
        stagingReceiptSha256: filePin(resolve(scenario, 'seed-stage-receipt.private.json')).sha256 };
    const bounds = { maxScenarioActionsIncludingActivations: 8, maxProxyInvocations: 3,
        proxyTimeoutMs: 180000, ownedGameHardTimeoutMs: 480000 };
    const historyBaseline = structuredClone(options.historyBaseline ?? { overallAttempts: 3, uiAttempts: 2, reservedActions: 0, sentActions: 0 });
    const campaignControl = options.generation ? { schemaVersion: 3, profile: 'GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE' }
        : options.inspection ? { schemaVersion: 2, profile: 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE' }
        : { schemaVersion: 1, profile: 'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE' };
    const plan = { scenarioId: basename(scenario), paths: { scenario }, ...(!noSeed ? { seed } : {}), bounds, historyBaseline, campaignControl,
        callerAdmissionDeadlineMs,
        fixtureBinding: { path: f.bindingPath, sha256: filePin(f.bindingPath).sha256 },
        ownership: { driver: '/root/recovery_game_driver', onlyGameInputDriver: true },
        batches: options.inspection ? [{ id: 'B1' }, { id: 'E1' }] : [{ id: 'B1' }, { id: 'E1' }, { id: 'E2' }] };
    json(planPath, plan);
    const supportPinsPath = resolve(scenario, 'frozen-support-pins.private.json');
    json(supportPinsPath, { pins: campaignLaunchSources.map(name => {
        const path = resolve(repository, name); return { path, sha256: filePin(path).sha256 };
    }) });
    const launchLease = { schemaVersion: 1, scenarioId: basename(scenario), authorized: true,
        driver: '/root/recovery_game_driver', issuer: '/root', phase: 'CAMPAIGN_BINDING_ONLY', planSha256: filePin(planPath).sha256,
        fixtureReceiptSha256: filePin(f.frozenPath).sha256, ...(!noSeed ? { seed } : {}), campaignControl,
        ownedGameHardTimeoutMs: 480000, maxActions: 8, maxProxyInvocations: 3, campaignLoadSaveAllowed: false,
        supportPinsPath, supportPinsSha256: filePin(supportPinsPath).sha256, callerAdmissionDeadlineMs };
    json(leasePath, launchLease);
    const now = Date.now();
    const controllerPath = resolve(scenario, 'controller-start.private.json');
    const controller = { at: new Date(startMs + 100).toISOString(), controllerPid: 100, sessionId: 1,
        controllerCreationTime: new Date(startMs + 100).toISOString(),
        controllerKernelCreationFileTime: '134371266963297182', sessionWatchdogMs: 480000,
        conservativeOuterDeadlineUtc: new Date(deadlineMs).toISOString(),
        controllerDeadlineUtc: new Date(deadlineMs).toISOString(),
        supervisorJobPids: [100, 101], hardTimeoutMs: 480000, installation: f.installation,
        fixtureReceiptSha256: launchLease.fixtureReceiptSha256, rootLeaseSha256: filePin(leasePath).sha256 };
    json(controllerPath, controller);
    const rootClock = { recordKey: 'generated-original-campaign-call', startMs, deadlineMs,
        sessionWatchdogMs: 480000, cleanupReserveMs: 30000, callerAdmissionDeadlineMs,
        originalCallerAdmissionDeadlineMs: callerAdmissionDeadlineMs, producerRecordKey: 'generated-controller',
        effectiveDeadlineMs: callerAdmissionDeadlineMs };
    const originalLaunchPath = resolve(scenario, 'campaign-postlaunch.private.json');
    const predecessor = { schemaVersion: 1, kind: 'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR',
        plan: { path: planPath, sha256: filePin(planPath).sha256 },
        lease: { path: leasePath, sha256: filePin(leasePath).sha256 },
        controller: { path: controllerPath, sha256: filePin(controllerPath).sha256 } };
    const campaign = { flow: 'PAUSED_CAMPAIGN_EXIT_NO_SAVE', windowId: 19, rootClock,
        producerWitness: { producerRecordKey: rootClock.producerRecordKey, controllerPid: 100,
            controllerCreationTime: controller.at, controllerKernelCreationFileTime: '134371266963297182',
            conservativeOuterDeadlineUtc: new Date(deadlineMs).toISOString(),
            controllerDeadlineUtc: new Date(deadlineMs).toISOString(), effectiveDeadlineMs: callerAdmissionDeadlineMs,
            sessionWatchdogMs: 480000 },
        rootCallRecord: { key: rootClock.recordKey, value: { startMs,
            args: { cmd: 'GENERATED_NOT_RUN' }, raw: { session_id: 777 } } },
        historyBaseline,
        maxReservedActions: historyBaseline.reservedActions + 8,
        launchPredecessor: { path: originalLaunchPath, sha256: '' } };
    Object.assign(predecessor, { rootClock, producerWitness: campaign.producerWitness,
        rootCallRecord: campaign.rootCallRecord, historyBaseline, maxReservedActions: campaign.maxReservedActions });
    campaign.launchPredecessor.sha256 = json(originalLaunchPath, predecessor).sha256;
    const phasePlanPath = resolve(scenario, 'E1-plan.private.json');
    const phaseLeasePath = resolve(scenario, 'E1-lease.private.json');
    const directHostConsent = { schemaVersion: 1, mode: 'finite-explicit-consent', maxCallsPerBatch: 32 };
    const phasePlan = { scenarioId: basename(scenario), paths: { scenario }, bounds, campaign,
        batches: [{ id: 'E1' }], directHostConsent };
    json(phasePlanPath, phasePlan);
    const phaseLease = { ...launchLease, schemaVersion: 2, phase: campaign.flow, issuer: '/root',
        driver: '/root/recovery_game_driver', planSha256: filePin(phasePlanPath).sha256, campaign, directHostConsent };
    delete /** @type {any} */ (phaseLease).seed;
    json(phaseLeasePath, phaseLease);
    const ownedReceiptPath = resolve(scenario, 'driver-E1-owned.private.json');
    json(ownedReceiptPath, { processes: [{ pid: 101, inSupervisorJob: true, supervisorJobPids: [100, 101],
        sessionId: 1, creationTime: new Date(startMs + 200).toISOString(),
        executable: resolve(f.installation, 'jre/bin/java.exe') }] });
    const source = resolve(scenario, 'E1-authorization.private.json');
    const auth = { schemaVersion: 2, scenario, batch: 'E1', planPath: phasePlanPath,
        planSha256: filePin(phasePlanPath).sha256, leasePath: phaseLeasePath,
        leaseSha256: filePin(phaseLeasePath).sha256, ownedReceiptPath,
        ownedReceiptSha256: filePin(ownedReceiptPath).sha256 };
    json(source, auth);
    json(resolve(scenario, 'guard-launch-history.private.json'), { schemaVersion: 2,
        kind: 'STARSECTOR_FINITE_LAUNCH_HISTORY', rootRecordKey: rootClock.recordKey,
        controllerSha256: filePin(controllerPath).sha256.toLowerCase(), history: { ...campaign.historyBaseline },
        pending: null, uncertain: false });
    function refreshLaunch() {
        launchLease.planSha256 = json(planPath, plan).sha256;
        controller.rootLeaseSha256 = json(leasePath, launchLease).sha256;
        json(controllerPath, controller);
        predecessor.plan.sha256 = filePin(planPath).sha256;
        predecessor.lease.sha256 = filePin(leasePath).sha256;
        predecessor.controller.sha256 = filePin(controllerPath).sha256;
        campaign.launchPredecessor.sha256 = json(originalLaunchPath, predecessor).sha256;
        auth.planSha256 = phaseLease.planSha256 = json(phasePlanPath, phasePlan).sha256;
        auth.leaseSha256 = json(phaseLeasePath, phaseLease).sha256;
        json(source, auth);
        json(resolve(scenario, 'guard-launch-history.private.json'), { schemaVersion: 2,
            kind: 'STARSECTOR_FINITE_LAUNCH_HISTORY', rootRecordKey: rootClock.recordKey,
            controllerSha256: filePin(controllerPath).sha256.toLowerCase(), history: { ...campaign.historyBaseline },
            pending: null, uncertain: false });
    }
    return { ...f, scenario, planPath, leasePath, phasePlanPath, phaseLeasePath, originalLaunchPath,
        predecessor, campaign, source, plan, launchLease, controller, controllerPath, refreshLaunch, phasePlan, phaseLease,
        root: repository, now, runID: 'immutable-campaign-phase',
        watchdog() { return powershell(['-Command', '. ' + quote(resolve(repository,
            'tools/starsector-proxy/fixture/session-watchdog.ps1')) + '; Read-NativeSessionWatchdog ' + quote(scenario) + ' 480000']); },
        admit() { return powershell(['-File', resolve(repository, 'tools/starsector-proxy/fixture/protection-check.ps1'),
            '-BindingPath', f.bindingPath, '-BindingSha256', f.bindingPin.sha256,
            '-Phase', 'admit-launch', '-EvidenceRoot', scenario]); } };
}
