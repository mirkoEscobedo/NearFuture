// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, openSync, closeSync, fsyncSync, renameSync, unlinkSync, existsSync } from 'node:fs';
import { resolve, basename, isAbsolute } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { fixtureProbeSources, readFixtureBinding } from './fixture-binding.mjs';
import { completePng } from './direct-frames.mjs';
import { requireThat, file, read, closed, inside, knownProviderClosure } from './campaign-proof.mjs';
import { inspectGenerationControl, generatedNameHandoff, assertInspectionSaves, generationActionLimit } from './generation-proof.mjs';
import { seedCreationFlow, seedCreationActionLimit } from './seed-creation-history.mjs';
import { seedWizardInput, seedWizardHandoff } from './seed-wizard-proof.mjs';
import { assertSeedCreationOutput } from './seed-creation-output.mjs';

export const campaignLaunchSources = Object.freeze([...fixtureProbeSources,
    ...['campaign-authority.mjs', 'batch-authorization.mjs', 'batch-session.mjs', 'finite-consent.mjs',
        'fixture-binding.mjs', 'direct-frames.mjs', 'campaign-proof.mjs', 'generation-proof.mjs',
        'seed-creation-history.mjs', 'seed-failed-attempt.mjs', 'seed-known-play-attempt.mjs', 'seed-wrong-scene-attempt.mjs', 'seed-capture-linkage-attempt.mjs', 'seed-caller-date-attempt.mjs', 'seed-caller-date-geometry.mjs', 'seed-zero-provider-attempt.mjs', 'seed-zero-provider-terminal.mjs', 'client-coordinate.mjs', 'seed-wizard-proof.mjs', 'seed-creation-output.mjs', 'seed-creation-custody.mjs'].map(name => 'tools/starsector-proxy/' + name)]);

const flow = 'PAUSED_CAMPAIGN_EXIT_NO_SAVE';
const seedFlow = 'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE';
const inspectionFlow = 'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE';
const generationFlow = 'GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE';
/** @param {any} value */
const exitConfirmationProfile = value => closed(value, ['schemaVersion', 'dialogText', 'confirmLabel']) &&
    value.schemaVersion === 1 && value.dialogText === 'Exit game?' && value.confirmLabel === 'Yes';
/** @param {any} binding */
const owner = binding => ({ batch: binding.batch, runID: binding.runID, sourceSha256: binding.sourceSha256 });
/** @param {any} binding */
const historyPath = binding => resolve(binding.scenario, 'guard-launch-history.private.json');

/** The root initializes this launch ledger. A provider cannot create a new origin.
 * @param {any} binding */
function originalHistory(binding) {
    const receipt = read(historyPath(binding)), state = receipt.value;
    const baseline = binding.campaign.historyBaseline;
    requireThat(state.schemaVersion === 2 && state.kind === 'STARSECTOR_FINITE_LAUNCH_HISTORY' &&
        state.rootRecordKey === binding.campaign.rootClock.recordKey && state.controllerSha256 === binding.controllerSha256,
    'original launch history required');
    for (const key of ['overallAttempts', 'uiAttempts', 'reservedActions', 'sentActions']) {
        requireThat(Number.isSafeInteger(baseline?.[key]) && baseline[key] >= 0 &&
            Number.isSafeInteger(state.history?.[key]) && state.history[key] >= baseline[key], 'inherited action history required');
    }
    requireThat(state.history.overallAttempts === baseline.overallAttempts && state.history.uiAttempts === baseline.uiAttempts &&
        state.history.sentActions <= state.history.reservedActions &&
        state.history.reservedActions <= binding.campaign.maxReservedActions && typeof state.uncertain === 'boolean',
    'original absolute history bounds required');
    requireThat(!state.uncertain, 'uncertain launch input is latched');
    return receipt;
}
/** @param {any} binding @param {number} [now] */
function history(binding, now = Date.now()) {
    const receipt = originalHistory(binding), state = receipt.value;
    requireThat(!state.provider || isDeepStrictEqual(state.provider, owner(binding)), 'another provider owns unresolved launch input');
    requireThat(!state.pending || isDeepStrictEqual(owner(state.pending), owner(binding)), 'unresolved launch action belongs to another owner');
    if (state.provider) {
        const c = state.providerClock, root = binding.campaign.rootClock;
        requireThat(Number.isSafeInteger(c?.startMs) && c.startMs >= root.startMs && c.startMs <= now &&
            c.deadlineMs === Math.min(c.startMs + 180000, root.effectiveDeadlineMs - root.cleanupReserveMs) &&
            c.deadlineMs - now >= 20000, 'original clipped provider deadline expired');
    }
    return receipt;
}

/** One new-profile-only, known-closed launcher-to-game window transfer.
 * @param {any} binding @param {any} reference @param {number} now @param {boolean} [historical] */
function seedLauncherHandoff(binding, reference, now, historical = false) {
    const firstPage = [4, 5, 6].includes(binding.schemaVersion);
    const state = originalHistory(binding).value;
    const sourcePath = resolve(binding.scenario, 'B1-authorization.private.json');
    const source = read(sourcePath), claim = read(resolve(binding.scenario, 'guard-B1-consumed.private.json'));
    const initial = claim.value.binding;
    requireThat(state.b1ClaimSha256 === claim.sha256 && claim.value.sourcePath === sourcePath &&
        claim.value.sourceSha256 === source.sha256 && initial?.schemaVersion === binding.schemaVersion && [3, 4, 5, 6].includes(initial.schemaVersion) && initial.batch === 'B1' &&
        initial.sourcePath === sourcePath && initial.sourceSha256 === source.sha256 &&
        initial.planSha256 === source.value.planSha256?.toLowerCase() && initial.controllerSha256 === binding.controllerSha256 &&
        initial.scenario === binding.scenario && initial.workspaceRoot === binding.workspaceRoot &&
        initial.originalPlanSha256 === binding.originalPlanSha256 &&
        isDeepStrictEqual({ ...initial.campaign, windowId: binding.campaign.windowId }, binding.campaign),
    'original seeded B1 lineage required');
    for (const [key, name] of [['planPath', 'B1-plan.private.json'], ['leasePath', 'B1-lease.private.json'],
        ['ownedReceiptPath', 'driver-B1-owned.private.json']]) {
        requireThat(source.value[key] === resolve(binding.scenario, name) &&
            read(source.value[key]).sha256 === source.value[key.replace('Path', 'Sha256')]?.toLowerCase(),
        'original seeded B1 custody changed');
    }
    const originalOwned = read(source.value.ownedReceiptPath).value.processes;
    const currentSource = read(binding.sourcePath).value;
    const currentOwned = read(currentSource.ownedReceiptPath).value.processes;
    const controller = read(resolve(binding.scenario, 'controller-start.private.json')).value;
    requireThat(Array.isArray(originalOwned) && Array.isArray(currentOwned), 'seeded owned process lineage required');
    for (const current of currentOwned) {
        const original = originalOwned.find((/** @type {any} */ row) => row.pid === current.pid);
        requireThat(original && initial.ownedPids.includes(current.pid) &&
            original.creationTime === current.creationTime && original.sessionId === current.sessionId &&
            current.sessionId === controller.sessionId && typeof original.executable === 'string' &&
            resolve(original.executable).toLowerCase() === resolve(current.executable).toLowerCase() &&
            (original.kernelCreationFileTime === undefined || original.kernelCreationFileTime === current.kernelCreationFileTime) &&
            (original.parentPid === undefined || original.parentPid === current.parentPid) &&
            [original, current].every(row => row.inSupervisorJob === true && Array.isArray(row.supervisorJobPids) &&
                row.supervisorJobPids.includes(current.pid) && row.supervisorJobPids.includes(controller.controllerPid)),
        'seeded owned process lineage changed');
    }
    requireThat(!historical || ([3, 5, 6].includes(binding.schemaVersion) && binding.batch === 'E1'), 'historical seeded entry binding required');
    const alreadyTransferred = historical || isDeepStrictEqual(state.provider, owner(binding));
    const action = alreadyTransferred ? state.resolvedB1 : state.pending;
    const providerClock = alreadyTransferred ? state.resolvedB1?.providerClock : state.providerClock;
    requireThat(action?.status === 'AWAITING_AFTER' && action.tool === 'left_click' &&
        isDeepStrictEqual(owner(action), owner(initial)) && action.args?.target_window_id === initial.campaign.windowId &&
        (alreadyTransferred || isDeepStrictEqual(state.provider, owner(initial))), 'seeded Play reservation proof required');
    const path = resolve(binding.scenario, 'B1-menu-inspected.private.json');
    requireThat(closed(reference, ['path', 'sha256']) && reference.path === path, 'seeded menu proof reference required');
    const receipt = read(path), value = receipt.value;
    requireThat(receipt.sha256 === reference.sha256 && closed(value, ['schemaVersion', 'kind', 'scenario', 'flow',
        'rootRecordKey', 'controllerSha256', 'b1', 'frame', 'inspection', 'provider']) && value.schemaVersion === 1 &&
        value.kind === 'STARSECTOR_B1_MAIN_MENU_INSPECTION' && value.scenario === binding.scenario && value.flow === binding.campaign.flow &&
        value.rootRecordKey === binding.campaign.rootClock.recordKey && value.controllerSha256 === binding.controllerSha256 &&
        closed(value.b1, ['sourceSha256', 'claimSha256', 'runID', 'reservationID']) &&
        value.b1.sourceSha256 === source.sha256 && value.b1.claimSha256 === claim.sha256 &&
        value.b1.runID === initial.runID && value.b1.reservationID === action.reservationID &&
        (!alreadyTransferred || action.menuReceiptSha256 === receipt.sha256), 'seeded Play reservation proof changed');
    const frame = value.frame, inspection = value.inspection;
    const pngPath = resolve(binding.scenario, 'B1-after/F1.original.png');
    const metadataPath = resolve(binding.scenario, 'B1-after/capture-001.metadata.private.json');
    const rawPath = resolve(binding.scenario, 'B1-after/capture-001.raw.private.json');
    requireThat(closed(frame, ['pngPath', 'pngSha256', 'metadataPath', 'metadataSha256', 'capturedAtMs']) &&
        frame.pngPath === pngPath && frame.metadataPath === metadataPath, 'seeded main-menu frame paths changed');
    const png = file(pngPath, 32 * 1024 * 1024), metadata = read(metadataPath), m = metadata.value;
    const raw = read(rawPath, 32 * 1024 * 1024), dimensions = completePng(png.bytes);
    const images = raw.value.content?.filter((/** @type {any} */ row) => row.type === 'image');
    requireThat(frame.pngSha256 === png.sha256 && frame.metadataSha256 === metadata.sha256 &&
        raw.value.isError !== true && images?.length === 1 && images[0].mimeType === 'image/png' &&
        typeof images[0].data === 'string' && png.bytes.toString('base64') === images[0].data &&
        m.captureReceipt === true && m.rawPath === rawPath && m.rawSha256 === raw.sha256 &&
        m.pngPath === pngPath && m.pngSha256 === png.sha256 && m.runID === initial.runID && m.batch === 'B1' &&
        m.sourceSha256 === source.sha256 && m.reservationID === action.reservationID && m.capturedAtMs === frame.capturedAtMs &&
        m.args?.target_window_id === binding.campaign.windowId && m.args.quality === 0 &&
        m.window?.windowId === binding.campaign.windowId && binding.ownedPids.includes(m.window.pid) &&
        m.window.isFocused === true && m.args.width === m.window.bounds?.width &&
        dimensions.width === m.window.bounds.width && dimensions.height === m.window.bounds.height &&
        isDeepStrictEqual(m.dimensions, dimensions), 'seeded main-menu original custody changed');
    const entryField = firstPage ? 'newGameCoordinate' : 'loadCoordinate', coordinate = inspection?.[entryField];
    requireThat(closed(inspection, ['issuer', 'driver', 'inspectedAtMs', 'state', entryField]) &&
        inspection.issuer === '/root' && inspection.driver === '/root/recovery_game_driver' &&
        inspection.state === 'STARSECTOR_MAIN_MENU' && Array.isArray(coordinate) &&
        coordinate.length === 2 && coordinate.every((/** @type {any} */ n) => Number.isSafeInteger(n)) &&
        coordinate[0] >= m.window.bounds.x && coordinate[0] < m.window.bounds.x + m.window.bounds.width &&
        coordinate[1] >= m.window.bounds.y && coordinate[1] < m.window.bounds.y + m.window.bounds.height,
    'seeded main-menu inspection required');
    const closedAt = knownProviderClosure({ binding, initial, initialSourceSha256: source.sha256,
        proof: value, providerClock, action, now });
    return { receiptSha256: receipt.sha256, loadCoordinate: coordinate, action, providerClock, closedAt, alreadyTransferred };
}

/** Read-only historical source custody for the modern seed consumer, no live authority.
 * @param {any} binding */
export function inspectSeedCreationLauncher(binding) {
    requireThat(binding.schemaVersion===6&&binding.batch==='E1','historical seed creation entry required');
    return seedLauncherHandoff(binding,{path:binding.b1MenuReceipt.path,sha256:binding.b1MenuReceipt.sha256},Date.now(),true);
}

/** Qualify one known-closed E1 handoff. Past provider time is historical evidence,
 * not a live E2 deadline. No unknown input or shutdown has a transfer path.
 * @param {any} binding @param {any} reference @param {number} now */
function campaignHandoff(binding, reference, now) {
    const rawHistory = read(historyPath(binding));
    requireThat(rawHistory.value.uncertain === false, 'uncertain launch input is latched');
    const sourcePath = resolve(binding.scenario, 'E1-authorization.private.json');
    const initialSource = read(sourcePath), claim = read(resolve(binding.scenario, 'guard-E1-consumed.private.json'));
    const initial = claim.value.binding;
    requireThat(rawHistory.value.initialClaimSha256 === claim.sha256 && claim.value.sourcePath === sourcePath &&
        claim.value.sourceSha256 === initialSource.sha256 && initial?.schemaVersion === binding.schemaVersion &&
        initial.planSha256 === initialSource.value.planSha256?.toLowerCase() && initial.sourceSha256 === initialSource.sha256 && initial.sourcePath === sourcePath && initial.batch === 'E1' &&
        initial.scenario === binding.scenario && initial.workspaceRoot === binding.workspaceRoot &&
        initial.controllerSha256 === binding.controllerSha256 && isDeepStrictEqual(initial.ownedPids, binding.ownedPids) &&
        isDeepStrictEqual(initial.campaign, binding.campaign), 'initial campaign lineage changed');
    const originalOwnedPath = resolve(binding.scenario, 'driver-E1-owned.private.json');
    requireThat(typeof initialSource.value.ownedReceiptPath === 'string' &&
        resolve(initialSource.value.ownedReceiptPath) === originalOwnedPath, 'initial owned identity custody path changed');
    const currentSource = read(resolve(binding.scenario, 'E2-authorization.private.json'));
    const currentOwnedPath = resolve(binding.scenario, 'driver-E2-owned.private.json');
    requireThat(currentSource.sha256 === binding.sourceSha256 && typeof currentSource.value.ownedReceiptPath === 'string' &&
        resolve(currentSource.value.ownedReceiptPath) === currentOwnedPath, 'initial owned identity current source changed');
    const currentOwned = read(currentOwnedPath), owned = currentOwned.value;
    requireThat(currentOwned.sha256 === currentSource.value.ownedReceiptSha256?.toLowerCase(), 'initial owned identity current custody changed');
    const originalOwned = read(originalOwnedPath), controller = read(resolve(binding.scenario, 'controller-start.private.json')).value;
    requireThat(originalOwned.sha256 === initialSource.value.ownedReceiptSha256?.toLowerCase() &&
        Array.isArray(originalOwned.value.processes) && originalOwned.value.processes.length === owned.processes.length,
    'initial owned identity custody changed');
    originalOwned.value.processes.forEach((/** @type {any} */ original, /** @type {number} */ index) => {
        const current = owned.processes[index];
        requireThat(original.pid === initial.ownedPids[index] && current.pid === original.pid &&
            current.creationTime === original.creationTime && typeof original.executable === 'string' &&
            resolve(current.executable).toLowerCase() === resolve(original.executable).toLowerCase() &&
            current.sessionId === original.sessionId && original.sessionId === controller.sessionId &&
            original.inSupervisorJob === true && current.inSupervisorJob === true &&
            [original, current].every(row => Array.isArray(row.supervisorJobPids) &&
                row.supervisorJobPids.includes(original.pid) && row.supervisorJobPids.includes(controller.controllerPid)) &&
            (original.kernelCreationFileTime === undefined || current.kernelCreationFileTime === original.kernelCreationFileTime) &&
            (original.parentPid === undefined || current.parentPid === original.parentPid),
        'initial owned identity changed');
    });
    const state = originalHistory(binding).value, alreadyTransferred = isDeepStrictEqual(state.provider, owner(binding));
    const action = alreadyTransferred ? state.resolvedE1 : state.pending;
    const providerClock = alreadyTransferred ? state.resolvedE1?.providerClock : state.providerClock;
    if ([5, 6].includes(binding.schemaVersion)) {
        for (const [key, name] of [['planPath', 'E1-plan.private.json'], ['leasePath', 'E1-lease.private.json']]) {
            requireThat(initialSource.value[key] === resolve(binding.scenario, name) &&
                read(initialSource.value[key]).sha256 === initialSource.value[key.replace('Path', 'Sha256')]?.toLowerCase(),
            'historical generation phase custody changed');
        }
        seedLauncherHandoff(initial, {path: initial.b1MenuReceipt.path, sha256: initial.b1MenuReceipt.sha256}, now, true);
    }
    if (binding.schemaVersion === 3) {
        for (const [key, name] of [['planPath', 'E1-plan.private.json'], ['leasePath', 'E1-lease.private.json']]) {
            requireThat(initialSource.value[key] === resolve(binding.scenario, name) &&
                read(initialSource.value[key]).sha256 === initialSource.value[key.replace('Path', 'Sha256')]?.toLowerCase(),
            'historical campaign phase custody changed');
        }
        seedLauncherHandoff(initial, { path: initial.b1MenuReceipt.path, sha256: initial.b1MenuReceipt.sha256 }, now, true);
        const entry = state.resolvedActions?.filter((/** @type {any} */ row) => isDeepStrictEqual(owner(row), owner(initial)));
        requireThat(entry?.length === 3 && entry.every((/** @type {any} */ row) => row.tool === 'left_click' && row.status === 'AWAITING_AFTER') &&
            entry[2].pauseInspectionSha256 === read(resolve(binding.scenario, 'E1-seed-paused-inspected.private.json')).sha256 &&
            action?.tool === 'key' && action.args?.text === 'escape' && action.args.repeat === 1,
        'original paused seed entry proof changed');
        const pausedPng = file(resolve(binding.scenario, 'E1-seed-paused/F1.original.png'), 32 * 1024 * 1024);
        requireThat(pausedPng.sha256 === entry[2].after?.pngSha256 &&
            seedSceneInspection(initial, entry[2], { ...entry[2].after, pngBytes: pausedPng.bytes }, now) === entry[2].pauseInspectionSha256,
        'original paused seed image custody changed');
    }
    const expectedOwner = owner(initial);
    requireThat(action?.status === 'AWAITING_AFTER' && isDeepStrictEqual(owner(action), expectedOwner) &&
        (alreadyTransferred || isDeepStrictEqual(state.provider, expectedOwner)), 'menu reservation proof is unresolved');
    if (binding.schemaVersion === 6) return seedWizardHandoff({binding, reference, now, initial, initialSource,
        claim, state, action, providerClock, alreadyTransferred});
    if (binding.schemaVersion === 5) return generatedNameHandoff({binding, reference, now, initial, initialSource,
        claim, state, action, providerClock, alreadyTransferred});
    const path = resolve(binding.scenario, 'E1-menu-inspected.private.json');
    requireThat(closed(reference, ['path', 'sha256']) && reference.path === path, 'menu reservation proof reference required');
    const receipt = read(path), value = receipt.value;
    requireThat(receipt.sha256 === reference.sha256 && closed(value, ['schemaVersion', 'kind', 'scenario', 'flow',
        'rootRecordKey', 'controllerSha256', 'e1', 'frame', 'inspection', 'provider']) && value.schemaVersion === 1 &&
        value.kind === 'STARSECTOR_E1_MENU_INSPECTION' && value.scenario === binding.scenario && value.flow === binding.campaign.flow &&
        value.rootRecordKey === binding.campaign.rootClock.recordKey && value.controllerSha256 === binding.controllerSha256 &&
        closed(value.e1, ['sourceSha256', 'claimSha256', 'runID', 'reservationID']) &&
        value.e1.sourceSha256 === initialSource.sha256 && value.e1.claimSha256 === claim.sha256 &&
        value.e1.runID === initial.runID && value.e1.reservationID === action.reservationID &&
        (!alreadyTransferred || action.menuReceiptSha256 === receipt.sha256), 'menu reservation proof mismatch');
    const frame = value.frame, inspection = value.inspection;
    const pngPath = resolve(binding.scenario, 'E1-after/F1.original.png');
    const metadataPath = resolve(binding.scenario, 'E1-after/capture-001.metadata.private.json');
    requireThat(closed(frame, ['pngPath', 'pngSha256', 'metadataPath', 'metadataSha256', 'capturedAtMs']) &&
        frame.pngPath === pngPath && frame.metadataPath === metadataPath, 'menu frame custody paths changed');
    const png = file(pngPath, 32 * 1024 * 1024), metadata = read(metadataPath), m = metadata.value;
    const rawPath = resolve(binding.scenario, 'E1-after/capture-001.raw.private.json'), raw = read(rawPath, 32 * 1024 * 1024);
    const images = raw.value.content?.filter((/** @type {any} */ item) => item.type === 'image');
    requireThat(png.sha256 === frame.pngSha256 && metadata.sha256 === frame.metadataSha256 &&
        m.captureReceipt === true && m.pngPath === pngPath && m.pngSha256 === png.sha256 && m.rawPath === rawPath &&
        m.rawSha256 === raw.sha256 && raw.value.isError !== true && images?.length === 1 && images[0].mimeType === 'image/png' &&
        typeof images[0].data === 'string' && Buffer.from(images[0].data, 'base64').equals(png.bytes) &&
        m.runID === initial.runID && m.batch === 'E1' && m.sourceSha256 === initialSource.sha256 &&
        m.reservationID === action.reservationID && m.capturedAtMs === frame.capturedAtMs &&
        m.args?.target_window_id === binding.campaign.windowId && m.args.quality === 0 &&
        m.window?.windowId === binding.campaign.windowId && binding.ownedPids.includes(m.window.pid) &&
        m.args.width === m.window.bounds?.width && m.dimensions?.width === m.window.bounds.width &&
        m.dimensions.height === m.window.bounds.height, 'menu frame custody changed');
    requireThat(closed(inspection, ['issuer', 'driver', 'inspectedAtMs', 'state', 'exitCoordinate']) &&
        inspection.issuer === '/root' && inspection.driver === '/root/recovery_game_driver' && inspection.state === 'ORDINARY_GAME_MENU' &&
        Array.isArray(inspection.exitCoordinate) && inspection.exitCoordinate.length === 2 &&
        inspection.exitCoordinate.every((/** @type {any} */ n) => Number.isSafeInteger(n)) &&
        inspection.exitCoordinate[0] >= m.window.bounds.x && inspection.exitCoordinate[0] < m.window.bounds.x + m.window.bounds.width &&
        inspection.exitCoordinate[1] >= m.window.bounds.y && inspection.exitCoordinate[1] < m.window.bounds.y + m.window.bounds.height,
    'menu reservation proof inspection required');
    const closedAt = knownProviderClosure({ binding, initial, initialSourceSha256: initialSource.sha256,
        proof: value, providerClock, action, now });
    return { receiptSha256: receipt.sha256, exitCoordinate: inspection.exitCoordinate, action, providerClock, closedAt, alreadyTransferred };
}

/** Reuse the capture contract's original-clock names and conservative minimum.
 * @param {any} campaign @param {any} controller @param {number} now */
function clock(campaign, controller, now, minimumRemainingMs = 20000) {
    const r = campaign.rootClock, p = campaign.producerWitness, call = campaign.rootCallRecord;
    const safe = Number.isSafeInteger, watchdog = r?.sessionWatchdogMs;
    requireThat(safe(now) && safe(watchdog) && watchdog >= 480000 && watchdog <= 43200000 &&
        safe(r.startMs) && r.startMs <= Date.parse(controller.at) && r.deadlineMs === r.startMs + watchdog &&
        r.cleanupReserveMs === 30000 && safe(r.callerAdmissionDeadlineMs) &&
        r.originalCallerAdmissionDeadlineMs === r.callerAdmissionDeadlineMs &&
        r.callerAdmissionDeadlineMs > r.startMs && r.callerAdmissionDeadlineMs <= r.deadlineMs &&
        controller.hardTimeoutMs === watchdog && p?.sessionWatchdogMs === watchdog,
    'original root clock required');
    const effective = Math.min(r.deadlineMs, r.callerAdmissionDeadlineMs,
        Date.parse(p.conservativeOuterDeadlineUtc), Date.parse(p.controllerDeadlineUtc));
    requireThat(safe(effective) && r.effectiveDeadlineMs === effective && p.effectiveDeadlineMs === effective &&
        now >= r.startMs && effective - r.cleanupReserveMs - now >= minimumRemainingMs &&
        p.producerRecordKey === r.producerRecordKey && p.controllerPid === controller.controllerPid &&
        p.controllerCreationTime === (campaign.launchPredecessor ? controller.controllerCreationTime : controller.at) &&
        /^[1-9][0-9]*$/.test(p.controllerKernelCreationFileTime) &&
        call?.key === r.recordKey && call.value?.startMs === r.startMs &&
        typeof call.value.args?.cmd === 'string' && call.value.args.cmd.length > 0 &&
        safe(call.value.raw?.session_id) && call.value.raw.session_id > 0, 'original effective deadline or producer custody invalid');
}

/** Resolve a phase without changing the plan/lease captured by the actual launcher.
 * @param {any} campaign @param {string} scenario @param {string} root @param {string} controllerSha256
 * @returns {string} */
function launchPredecessor(campaign, scenario, root, controllerSha256) {
    const inspection = campaign.flow === inspectionFlow, generation = campaign.flow === generationFlow,
        creation = campaign.flow === seedCreationFlow;
    /** @param {any} descriptor @param {string} name */
    function pinned(descriptor, name) {
        requireThat(closed(descriptor, ['path', 'sha256']) && typeof descriptor.path === 'string' &&
            isAbsolute(descriptor.path) && resolve(descriptor.path) === resolve(scenario, name) &&
            typeof descriptor.sha256 === 'string' && /^[a-f0-9]{64}$/i.test(descriptor.sha256), 'exact launch predecessor reference required');
        const found = read(descriptor.path);
        requireThat(found.sha256 === descriptor.sha256.toLowerCase(), 'launch predecessor bytes changed');
        return found;
    }
    const predecessor = pinned(campaign.launchPredecessor, 'campaign-postlaunch.private.json').value;
    requireThat(closed(predecessor, ['schemaVersion', 'kind', 'plan', 'lease', 'controller', 'rootClock',
        'producerWitness', 'rootCallRecord', 'historyBaseline', 'maxReservedActions']) && predecessor.schemaVersion === 1 &&
        predecessor.kind === 'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR', 'closed launch predecessor required');
    const planFile = pinned(predecessor.plan, 'plan.json'), leaseFile = pinned(predecessor.lease, 'native-lane-lease.private.json');
    const controllerFile = pinned(predecessor.controller, 'controller-start.private.json');
    const plan = planFile.value, lease = leaseFile.value, controller = controllerFile.value;
    requireThat(controllerFile.sha256 === controllerSha256 && controller.rootLeaseSha256?.toLowerCase() === leaseFile.sha256 &&
        lease.planSha256?.toLowerCase() === planFile.sha256, 'original launch authority changed');
    requireThat(plan.scenarioId === basename(scenario) && resolve(plan.paths?.scenario ?? '') === scenario &&
        plan.ownership?.driver === '/root/recovery_game_driver' && plan.ownership.onlyGameInputDriver === true &&
        plan.bounds?.maxScenarioActionsIncludingActivations === 8 && plan.bounds.maxProxyInvocations === 3 &&
        plan.bounds.proxyTimeoutMs === 180000 && plan.bounds.ownedGameHardTimeoutMs === 480000 &&
        Array.isArray(plan.batches) && plan.batches.length === (inspection ? 2 : 3) &&
        [...new Set(plan.batches.map((/** @type {any} */ batch) => batch.id))].sort().join() === (inspection ? 'B1,E1' : 'B1,E1,E2') &&
        lease.schemaVersion === 1 && lease.scenarioId === basename(scenario) && lease.authorized === true &&
        lease.issuer === '/root' && lease.driver === '/root/recovery_game_driver' && lease.phase === 'CAMPAIGN_BINDING_ONLY' &&
        lease.ownedGameHardTimeoutMs === 480000 && lease.maxActions === 8 && lease.maxProxyInvocations === 3 &&
        lease.campaignLoadSaveAllowed === false, 'closed original campaign launch scope required');
    requireThat(closed(plan.campaignControl, ['schemaVersion', 'profile']) && plan.campaignControl.schemaVersion === (creation ? 4 : generation ? 3 : inspection ? 2 : 1) &&
        plan.campaignControl.profile === (creation ? seedCreationFlow : generation ? generationFlow : inspection ? inspectionFlow : seedFlow) &&
        isDeepStrictEqual(plan.campaignControl, lease.campaignControl), 'closed seed entry launch profile required');
    requireThat(![plan, lease].some(value => ['ownedNodeKill', 'campaign', 'e1MenuReceipt'].some(key => Object.hasOwn(value, key))),
        'unrelated launch phase authority refused');
    const watchdogDeclared = Object.hasOwn(plan, 'sessionWatchdogMs');
    requireThat(watchdogDeclared === Object.hasOwn(lease, 'sessionWatchdogMs') &&
        (!watchdogDeclared || (plan.sessionWatchdogMs === 480000 && lease.sessionWatchdogMs === 480000)) &&
        campaign.rootClock.sessionWatchdogMs === 480000, 'paired original campaign watchdog required');
    for (const key of ['rootClock', 'producerWitness', 'rootCallRecord', 'historyBaseline', 'maxReservedActions']) {
        requireThat(isDeepStrictEqual(campaign[key], predecessor[key]), 'immutable postlaunch clock or history changed');
    }
    requireThat(isDeepStrictEqual(plan.historyBaseline, campaign.historyBaseline) &&
        campaign.maxReservedActions === (creation ? seedCreationActionLimit(plan, lease, scenario) : generation ? generationActionLimit(plan, lease, scenario)
            : plan.historyBaseline.reservedActions + 8), 'original global action budget required');
    requireThat(Number.isSafeInteger(plan.callerAdmissionDeadlineMs) && plan.callerAdmissionDeadlineMs > 0 &&
        lease.callerAdmissionDeadlineMs === plan.callerAdmissionDeadlineMs &&
        campaign.rootClock.callerAdmissionDeadlineMs === plan.callerAdmissionDeadlineMs &&
        campaign.rootClock.originalCallerAdmissionDeadlineMs === plan.callerAdmissionDeadlineMs,
    'original caller admission deadline changed');
    const birthMs = Date.parse(controller.controllerCreationTime), publishedMs = Date.parse(controller.at);
    const outerMs = Date.parse(controller.conservativeOuterDeadlineUtc), controllerMs = Date.parse(controller.controllerDeadlineUtc);
    const producer = campaign.producerWitness;
    requireThat([birthMs, publishedMs, outerMs, controllerMs].every(Number.isSafeInteger) &&
        campaign.rootClock.startMs <= birthMs && birthMs <= publishedMs && publishedMs < controllerMs &&
        controllerMs <= outerMs && outerMs <= birthMs + 480000 &&
        controller.sessionWatchdogMs === 480000 && producer.sessionWatchdogMs === controller.sessionWatchdogMs &&
        producer.controllerCreationTime === controller.controllerCreationTime &&
        typeof controller.controllerKernelCreationFileTime === 'string' &&
        /^[1-9][0-9]*$/.test(controller.controllerKernelCreationFileTime) &&
        producer.controllerKernelCreationFileTime === controller.controllerKernelCreationFileTime &&
        Date.parse(producer.conservativeOuterDeadlineUtc) === outerMs &&
        Date.parse(producer.controllerDeadlineUtc) === controllerMs,
    'actual controller clock and identity required');
    readFixtureBinding(/** @type {any} */ ({ binding: { scenario, planSha256: planFile.sha256 } }));
    const fixture = read(resolve(scenario, 'protected-fixture-receipt.private.json'));
    requireThat(lease.fixtureReceiptSha256?.toLowerCase() === fixture.sha256 &&
        controller.fixtureReceiptSha256?.toLowerCase() === fixture.sha256, 'original campaign fixture changed');
    if (inspection || generation || creation) {
        requireThat(!Object.hasOwn(plan, 'seed') && !Object.hasOwn(lease, 'seed'), 'first-page inspection refuses seed authority');
        for (const name of ['seed-manifest.private.json', 'seed-stage-plan.private.json', 'seed-stage-authority.private.json',
            'seed-stage-claim.private.json', 'seed-stage-receipt.private.json']) {
            requireThat(!existsSync(resolve(scenario, name)), 'first-page inspection refuses seed metadata');
        }
        if (inspection) assertInspectionSaves({scenario}, undefined, false, Date.now());
    } else {
    requireThat(closed(plan.seed, ['mode', 'manifestSha256', 'stagingReceiptSha256']) &&
        plan.seed.mode === 'LOAD_ONE_PRIVATE_DISPOSABLE_SEED' && isDeepStrictEqual(plan.seed, lease.seed), 'exact staged seed scope required');
    const manifest = read(resolve(scenario, 'seed-manifest.private.json'));
    const staged = read(resolve(scenario, 'seed-stage-receipt.private.json'));
    requireThat(plan.seed.manifestSha256?.toLowerCase() === manifest.sha256 &&
        plan.seed.stagingReceiptSha256?.toLowerCase() === staged.sha256, 'original staged seed changed');
    const save = manifest.value;
    requireThat(((save.schemaVersion===1&&save.kind==='NATIVE99_DISPOSABLE_SEED_MANIFEST_V1')||
        (save.schemaVersion===2&&save.kind==='NATIVE99_DISPOSABLE_SEED_MANIFEST_V2')) && save.gameVersion === '0.98a-RC8' &&
        typeof save.saveDirectory === 'string' && /^[A-Za-z0-9_-]+$/.test(save.saveDirectory) &&
        Array.isArray(save.files) && save.files.length === 2 &&
        save.files.map((/** @type {any} */ row) => row.relative).sort().join() === 'campaign.xml,descriptor.xml', 'closed manifest-bound seed files required');
    let seedBytes = 0;
    for (const row of save.files) {
        const current = file(resolve(scenario, 'installation/saves', save.saveDirectory, row.relative), 16 * 1024 * 1024);
        requireThat(current.sha256 === row.sha256?.toLowerCase() && current.bytes.length === row.bytes, 'staged seed file changed');
        seedBytes += current.bytes.length;
    }
    requireThat(seedBytes <= 16 * 1024 * 1024, 'bounded staged seed required');
    }
    requireThat(typeof lease.supportPinsPath === 'string' && resolve(lease.supportPinsPath) === resolve(scenario, 'frozen-support-pins.private.json'),
        'original campaign support manifest required');
    const support = read(lease.supportPinsPath);
    requireThat(lease.supportPinsSha256?.toLowerCase() === support.sha256 && Array.isArray(support.value.pins), 'original campaign support changed');
    for (const source of campaignLaunchSources) {
        const path = resolve(root, source), pins = support.value.pins.filter((/** @type {any} */ pin) => pin.path === path);
        requireThat(pins.length === 1 && pins[0].sha256?.toLowerCase() === file(path, 1024 * 1024).sha256,
            'current campaign launch tooling pin required');
    }
    return planFile.sha256;
}

/** Closed V2 admission for Escape and its known-closed ordinary Exit handoff.
 * @param {string} path @param {{root: string, runID: string, now?: number, input?: boolean}} options */
export function readCampaignAuthorization(path, { root, runID, now = Date.now(), input = false }) {
    const sourcePath = resolve(path), source = read(sourcePath), auth = source.value;
    const inspection = auth.schemaVersion === 4, generation = auth.schemaVersion === 5, creation = auth.schemaVersion === 6,
        seeded = [3, 4, 5, 6].includes(auth.schemaVersion);
    requireThat(closed(auth, ['batch', 'leasePath', 'leaseSha256', 'ownedReceiptPath', 'ownedReceiptSha256',
        'planPath', 'planSha256', 'scenario', 'schemaVersion']) && [2, 3, 4, 5, 6].includes(auth.schemaVersion) &&
        typeof auth.scenario === 'string' && isAbsolute(auth.scenario) && (generation ? ['B1', 'E1', 'E2'].includes(auth.batch) : inspection ? ['B1', 'E1'].includes(auth.batch) : seeded ? ['B1', 'E1', 'E2'].includes(auth.batch) : ['E1', 'E2'].includes(auth.batch)) &&
        typeof runID === 'string' && /^[A-Za-z0-9_-]+$/.test(runID), 'closed V2 authorization required');
    const scenario = resolve(auth.scenario);
    requireThat(inside(resolve(root, '.tmp/starsector-proxy/scenarios'), scenario) &&
        sourcePath === resolve(scenario, auth.batch + '-authorization.private.json'), 'exact campaign scenario required');
    /** @type {Record<string, ReturnType<typeof read>>} */ const files = {};
    const phaseLocal = typeof auth.planPath === 'string' && resolve(auth.planPath) === resolve(scenario, auth.batch + '-plan.private.json');
    requireThat(seeded || !phaseLocal, 'legacy campaign cannot use launch predecessor');
    requireThat(!seeded || phaseLocal, 'seed control requires fixed phase-local authority');
    for (const [key, expected] of Object.entries({ planPath: phaseLocal ? auth.batch + '-plan.private.json' : 'plan.json',
        leasePath: phaseLocal ? auth.batch + '-lease.private.json' : 'native-lane-lease.private.json',
        ownedReceiptPath: 'driver-' + auth.batch + '-owned.private.json' })) {
        requireThat(typeof auth[key] === 'string' && resolve(auth[key]) === resolve(scenario, expected), 'exact campaign custody path required');
        files[key] = read(auth[key]);
        requireThat(files[key].sha256 === auth[key.replace('Path', 'Sha256')]?.toLowerCase(), 'campaign custody hash changed');
    }
    const plan = files.planPath.value, lease = files.leasePath.value, campaign = plan.campaign;
    requireThat(!(generation || creation) || ![plan, lease].some(value => ['seed', 'exitConfirmation', 'e1MenuReceipt'].some(key => Object.hasOwn(value, key))),
        'generation refuses unrelated phase authority');
    requireThat(closed(campaign, ['flow', 'windowId', 'rootClock', 'producerWitness', 'rootCallRecord',
        'historyBaseline', 'maxReservedActions', ...(phaseLocal ? ['launchPredecessor'] : [])]) && campaign.flow === (creation ? seedCreationFlow : generation ? generationFlow : inspection ? inspectionFlow : seeded ? seedFlow : flow) &&
        Number.isSafeInteger(campaign.windowId) && campaign.windowId > 0 &&
        closed(campaign.historyBaseline, ['overallAttempts', 'uiAttempts', 'reservedActions', 'sentActions']) &&
        isDeepStrictEqual(campaign, lease.campaign),
    'closed matching campaign flow required');
    const controllerFile = read(resolve(scenario, 'controller-start.private.json')), controller = controllerFile.value;
    const originalPlanSha256 = phaseLocal ? launchPredecessor(campaign, scenario, root, controllerFile.sha256) : undefined;
    const owned = files.ownedReceiptPath.value;
    requireThat(Number.isSafeInteger(controller.controllerPid) && controller.controllerPid > 0 &&
        Number.isSafeInteger(controller.sessionId) && controller.sessionId > 0 &&
        controller.supervisorJobPids?.includes(controller.controllerPid) &&
        resolve(controller.installation ?? '') === resolve(scenario, 'installation') &&
        Array.isArray(owned.processes) && owned.processes.length > 0, 'owned controller required');
    const ownedPids = owned.processes.map((/** @type {any} */ process) => {
        requireThat(Number.isSafeInteger(process.pid) && process.pid > 0 && process.inSupervisorJob === true &&
            process.supervisorJobPids?.includes(process.pid) && process.sessionId === controller.sessionId &&
            typeof process.executable === 'string' && inside(resolve(scenario, 'installation'), process.executable) &&
            Number.isFinite(Date.parse(process.creationTime)) && Date.parse(process.creationTime) >= Date.parse(controller.at) &&
            Date.parse(process.creationTime) <= now, 'owned target receipt required');
        return process.pid;
    });
    const exitConfirmation = seeded && auth.batch === 'E2' ? plan.exitConfirmation : undefined;
    if (seeded && (plan.exitConfirmation !== undefined || lease.exitConfirmation !== undefined)) {
        requireThat(auth.batch === 'E2' && exitConfirmationProfile(exitConfirmation) &&
            isDeepStrictEqual(exitConfirmation, lease.exitConfirmation), 'closed matching exit confirmation declaration required');
    }
    const binding = { schemaVersion: auth.schemaVersion, batch: auth.batch, runID, scenario, workspaceRoot: resolve(root), planSha256: files.planPath.sha256,
        ownedPids, campaign, sourcePath, sourceSha256: source.sha256, controllerSha256: controllerFile.sha256,
        ...(originalPlanSha256 ? { originalPlanSha256 } : {}), ...(exitConfirmation ? { exitConfirmation } : {}) };
    let handoff;
    if (seeded && auth.batch === 'E1') {
        requireThat(isDeepStrictEqual(plan.b1MenuReceipt, lease.b1MenuReceipt), 'seeded menu grant changed');
        handoff = seedLauncherHandoff(binding, plan.b1MenuReceipt, now);
        if (handoff.alreadyTransferred) history(binding, now);
    } else if (auth.batch === 'E2') {
        const key = creation ? 'e1WizardReceipt' : generation ? 'e1GeneratedNameReceipt' : 'e1MenuReceipt';
        requireThat(isDeepStrictEqual(plan[key], lease[key]), 'entry reservation proof grant changed');
        handoff = campaignHandoff(binding, plan[key], now);
        if (handoff.alreadyTransferred) history(binding, now);
    } else history(binding, now);
    requireThat(plan.scenarioId === basename(scenario) && resolve(plan.paths?.scenario ?? '') === scenario &&
        plan.bounds?.maxScenarioActionsIncludingActivations === 8 && plan.bounds?.maxProxyInvocations === 3 &&
        plan.bounds?.proxyTimeoutMs === 180000 && plan.bounds?.ownedGameHardTimeoutMs === campaign.rootClock.sessionWatchdogMs &&
        plan.batches?.some((/** @type {any} */ batch) => batch.id === auth.batch) && lease.schemaVersion === auth.schemaVersion &&
        lease.authorized === true && lease.phase === (creation ? seedCreationFlow : generation ? generationFlow : inspection ? inspectionFlow : seeded ? seedFlow : flow) && lease.issuer === '/root' && lease.driver === '/root/recovery_game_driver' &&
        lease.planSha256?.toLowerCase() === files.planPath.sha256 && lease.campaignLoadSaveAllowed === false &&
        lease.ownedGameHardTimeoutMs === campaign.rootClock.sessionWatchdogMs && lease.maxActions === 8 && lease.maxProxyInvocations === 3 &&
        Number.isSafeInteger(campaign.maxReservedActions) &&
        campaign.maxReservedActions === (creation ? seedCreationActionLimit(plan, lease, scenario) : generation ? generationActionLimit(plan, lease, scenario)
            : campaign.historyBaseline.reservedActions + lease.maxActions),
    'current finite root campaign lease required');
    requireThat(resolve(lease.supportPinsPath ?? '') === resolve(scenario, 'frozen-support-pins.private.json') &&
        read(lease.supportPinsPath).sha256 === lease.supportPinsSha256?.toLowerCase() &&
        read(resolve(scenario, 'protected-fixture-receipt.private.json')).sha256 === lease.fixtureReceiptSha256?.toLowerCase() &&
        controller.fixtureReceiptSha256?.toLowerCase() === lease.fixtureReceiptSha256?.toLowerCase(), 'original support/fixture custody changed');
    clock(campaign, controller, now, input ? 120000 : 20000);
    const admitted = handoff ? 'loadCoordinate' in handoff
        ? { ...binding, b1MenuReceipt: { path: plan.b1MenuReceipt.path, sha256: handoff.receiptSha256,
            ...(inspection || generation || creation ? { newGameCoordinate: handoff.loadCoordinate } : { loadCoordinate: handoff.loadCoordinate }) } }
        : 'generatedName' in handoff ? { ...binding, e1WizardReceipt: { path: plan.e1WizardReceipt.path,
            sha256: handoff.receiptSha256, generatedName: handoff.generatedName } }
        : 'continueCoordinate' in handoff ? { ...binding, e1GeneratedNameReceipt: { path: plan.e1GeneratedNameReceipt.path,
            sha256: handoff.receiptSha256, continueCoordinate: handoff.continueCoordinate } }
        : { ...binding, e1MenuReceipt: { path: plan.e1MenuReceipt.path,
            sha256: handoff.receiptSha256, exitCoordinate: handoff.exitCoordinate } } : binding;
    if (generation) assertInspectionSaves(admitted, originalHistory(admitted).value, input, now);
    if (creation) assertSeedCreationOutput(admitted, originalHistory(admitted).value, input);
    return { sourcePath, sourceSha256: source.sha256, binding: admitted, claimPath: resolve(scenario, 'guard-' + auth.batch + '-consumed.private.json') };
}

/** @param {any} binding */
export function validateCampaignBinding(binding) {
    const inspection = binding?.schemaVersion === 4, generation = binding?.schemaVersion === 5,
        creation = binding?.schemaVersion === 6, seeded = [3, 4, 5, 6].includes(binding?.schemaVersion);
    requireThat(seeded || (!binding?.campaign?.launchPredecessor && binding?.originalPlanSha256 === undefined),
        'legacy campaign cannot use launch predecessor');
    const fields = ['schemaVersion', 'batch', 'runID', 'scenario', 'workspaceRoot', 'planSha256', 'ownedPids',
        'campaign', 'sourcePath', 'sourceSha256', 'controllerSha256'];
    if (binding?.batch === 'E2') fields.push(creation ? 'e1WizardReceipt' : generation ? 'e1GeneratedNameReceipt' : 'e1MenuReceipt');
    if (seeded && binding?.batch === 'E1') fields.push('b1MenuReceipt');
    if (binding?.campaign?.launchPredecessor) fields.push('originalPlanSha256');
    if (binding?.exitConfirmation !== undefined) fields.push('exitConfirmation');
    requireThat(closed(binding, fields) && [2, 3, 4, 5, 6].includes(binding.schemaVersion) &&
        (generation ? ['B1', 'E1', 'E2'].includes(binding.batch) : inspection ? ['B1', 'E1'].includes(binding.batch) : seeded ? ['B1', 'E1', 'E2'].includes(binding.batch) : ['E1', 'E2'].includes(binding.batch)) &&
        binding.campaign?.flow === (creation ? seedCreationFlow : generation ? generationFlow : inspection ? inspectionFlow : seeded ? seedFlow : flow) && (!seeded || binding.campaign.launchPredecessor) && Number.isSafeInteger(binding.campaign.windowId) &&
        typeof binding.workspaceRoot === 'string' && isAbsolute(binding.workspaceRoot) &&
        typeof binding.scenario === 'string' && inside(resolve(binding.workspaceRoot, '.tmp/starsector-proxy/scenarios'), binding.scenario) &&
        binding.sourcePath === resolve(binding.scenario, binding.batch + '-authorization.private.json') &&
        typeof binding.runID === 'string' && /^[A-Za-z0-9_-]+$/.test(binding.runID) &&
        [binding.planSha256, binding.sourceSha256, binding.controllerSha256].every(hash => typeof hash === 'string' && /^[a-fA-F0-9]{64}$/.test(hash)) &&
        Array.isArray(binding.ownedPids) && binding.ownedPids.length > 0 &&
        binding.ownedPids.every((/** @type {any} */ pid) => Number.isSafeInteger(pid) && pid > 0) &&
        new Set(binding.ownedPids).size === binding.ownedPids.length &&
        (binding.batch !== 'E2' || (creation ? closed(binding.e1WizardReceipt, ['path', 'sha256', 'generatedName']) : generation ? closed(binding.e1GeneratedNameReceipt, ['path', 'sha256', 'continueCoordinate']) : closed(binding.e1MenuReceipt, ['path', 'sha256', 'exitCoordinate']))) &&
        (!seeded || binding.batch !== 'E1' || closed(binding.b1MenuReceipt, ['path', 'sha256', inspection || generation || creation ? 'newGameCoordinate' : 'loadCoordinate'])) &&
        (binding.exitConfirmation === undefined || (!creation && !generation && seeded && binding.batch === 'E2' && exitConfirmationProfile(binding.exitConfirmation))) &&
        (!binding.campaign.launchPredecessor || (typeof binding.originalPlanSha256 === 'string' && /^[a-f0-9]{64}$/.test(binding.originalPlanSha256))),
    'closed campaign binding required');
}

/** Lock and exact-byte CAS protect one ordinary launch ledger, not a new clock.
 * @param {any} binding @param {(state: any) => void} change @param {'current'|'latch'|'handoff'} [mode] */
function update(binding, change, mode = 'current') {
    const path = historyPath(binding), lock = path + '.lock', next = path + '.next';
    const fd = openSync(lock, 'wx');
    try {
        if (mode === 'handoff') {
            const admitted = readCampaignAuthorization(binding.sourcePath, { root: binding.workspaceRoot, runID: binding.runID });
            requireThat(isDeepStrictEqual(admitted.binding, binding), 'atomic handoff custody changed');
        }
        const current = mode === 'latch' ? read(path) : mode === 'handoff' ? originalHistory(binding) : history(binding);
        change(current.value);
        const out = openSync(next, 'wx');
        try { writeFileSync(out, JSON.stringify(current.value) + '\n'); fsyncSync(out); } finally { closeSync(out); }
        requireThat(readFileSync(path).equals(current.bytes), 'launch history changed during reservation');
        renameSync(next, path);
    } finally { closeSync(fd); unlinkSync(lock); }
}

/** @param {any} binding */
export function claimCampaignOwner(binding) {
    if ([3, 4, 5, 6].includes(binding.schemaVersion) && binding.batch === 'E1') {
        update(binding, state => {
            const proof = seedLauncherHandoff(binding, { path: binding.b1MenuReceipt.path,
                sha256: binding.b1MenuReceipt.sha256 }, Date.now());
            requireThat(!proof.alreadyTransferred, 'seed entry provider already claimed');
            const startMs = Date.now(), root = binding.campaign.rootClock;
            state.resolvedB1 = { ...structuredClone(proof.action), providerClock: structuredClone(proof.providerClock),
                menuReceiptSha256: proof.receiptSha256, providerClosedAtMs: proof.closedAt, resolvedAtMs: startMs };
            state.initialClaimSha256 = read(resolve(binding.scenario, 'guard-E1-consumed.private.json')).sha256;
            state.provider = owner(binding); state.pending = null;
            state.providerClock = { startMs, deadlineMs: Math.min(startMs + 180000, root.effectiveDeadlineMs - root.cleanupReserveMs) };
        }, 'handoff');
        return;
    }
    if (binding.batch === 'E2') {
        update(binding, state => {
            const reference = binding.schemaVersion === 6 ? binding.e1WizardReceipt : binding.schemaVersion === 5 ? binding.e1GeneratedNameReceipt : binding.e1MenuReceipt;
            const proof = campaignHandoff(binding, { path: reference.path, sha256: reference.sha256 }, Date.now());
            requireThat(!proof.alreadyTransferred, 'E2 provider already claimed');
            const startMs = Date.now(), root = binding.campaign.rootClock;
            state.resolvedE1 = { ...structuredClone(proof.action), providerClock: structuredClone(proof.providerClock),
                menuReceiptSha256: proof.receiptSha256, providerClosedAtMs: proof.closedAt, resolvedAtMs: startMs };
            state.provider = owner(binding); state.pending = null;
            state.providerClock = { startMs, deadlineMs: Math.min(startMs + 180000, root.effectiveDeadlineMs - root.cleanupReserveMs) };
        }, 'handoff');
        return;
    }
    update(binding, state => {
        requireThat(!state.provider && !state.pending, 'launch provider already claimed');
        const startMs = Date.now(), root = binding.campaign.rootClock;
        state.provider = owner(binding);
        const claimSha256 = read(resolve(binding.scenario, 'guard-' + binding.batch + '-consumed.private.json')).sha256;
        if ([3, 4, 5, 6].includes(binding.schemaVersion) && binding.batch === 'B1') state.b1ClaimSha256 = claimSha256;
        else state.initialClaimSha256 = claimSha256;
        state.providerClock = { startMs, deadlineMs: Math.min(startMs + 180000, root.effectiveDeadlineMs - root.cleanupReserveMs) };
    });
}

/** The driver inspects the actual original; capture receipt alone never means pause.
 * @param {any} binding @param {any} previous @param {any} after @param {number} now @param {any} [args] */
function seedSceneInspection(binding, previous, after, now, args) {
    const confirmation = binding.batch === 'E2';
    const prefix = confirmation ? 'E2-exit-confirmation' : 'E1-seed-paused';
    const label = confirmation ? 'exit confirmation' : 'paused seed';
    const path = resolve(binding.scenario, prefix + '-inspected.private.json');
    requireThat(existsSync(path), label + ' inspection required before input');
    const receipt = read(path), value = receipt.value;
    const original = read(resolve(binding.scenario, 'plan.json')).value.seed;
    requireThat(closed(value, ['schemaVersion', 'kind', 'scenario', 'rootRecordKey', 'controllerSha256',
        'sourceSha256', 'claimSha256', 'runID', 'reservationID', 'manifestSha256', 'stagingReceiptSha256', 'frame', 'inspection']) &&
        value.schemaVersion === 1 && value.kind === (confirmation ? 'STARSECTOR_EXIT_CONFIRMATION_INSPECTION' : 'STARSECTOR_SEED_PAUSE_INSPECTION') && value.scenario === binding.scenario &&
        value.rootRecordKey === binding.campaign.rootClock.recordKey && value.controllerSha256 === binding.controllerSha256 &&
        value.sourceSha256 === binding.sourceSha256 && value.runID === binding.runID && value.reservationID === previous.reservationID &&
        value.claimSha256 === read(resolve(binding.scenario, 'guard-' + binding.batch + '-consumed.private.json')).sha256 &&
        value.manifestSha256 === original.manifestSha256?.toLowerCase() &&
        value.stagingReceiptSha256 === original.stagingReceiptSha256?.toLowerCase(), label + ' inspection lineage changed');
    const frame = value.frame, inspection = value.inspection;
    const pngPath = resolve(binding.scenario, prefix, 'F1.original.png');
    const metadataPath = resolve(binding.scenario, prefix, 'capture-001.metadata.private.json');
    const rawPath = resolve(binding.scenario, prefix, 'capture-001.raw.private.json');
    requireThat(closed(frame, ['pngPath', 'pngSha256', 'metadataPath', 'metadataSha256', 'capturedAtMs']) &&
        frame.pngPath === pngPath && frame.metadataPath === metadataPath, label + ' original paths changed');
    const png = file(pngPath, 32 * 1024 * 1024), metadata = read(metadataPath), m = metadata.value;
    const raw = read(rawPath, 32 * 1024 * 1024);
    const images = raw.value.content?.filter((/** @type {any} */ row) => row.type === 'image');
    const dimensions = completePng(png.bytes);
    requireThat(png.sha256 === frame.pngSha256 && metadata.sha256 === frame.metadataSha256 && png.bytes.equals(after.pngBytes) &&
        raw.value.isError !== true && images?.length === 1 && images[0].mimeType === 'image/png' &&
        png.bytes.toString('base64') === images[0].data && m.captureReceipt === true &&
        m.rawPath === rawPath && m.rawSha256 === raw.sha256 && m.pngPath === pngPath && m.pngSha256 === png.sha256 &&
        m.runID === binding.runID && m.batch === binding.batch && m.sourceSha256 === binding.sourceSha256 &&
        m.reservationID === previous.reservationID && m.capturedAtMs === frame.capturedAtMs &&
        m.args?.target_window_id === binding.campaign.windowId && m.args.quality === 0 && m.args.width === after.window.bounds.width &&
        m.window?.windowId === after.window.windowId && m.window.pid === after.window.pid && m.window.isFocused === true &&
        isDeepStrictEqual(m.window.bounds, after.window.bounds) && isDeepStrictEqual(m.dimensions, dimensions) &&
        dimensions.width === after.window.bounds.width && dimensions.height === after.window.bounds.height &&
        Number.isSafeInteger(frame.capturedAtMs) && frame.capturedAtMs >= after.capturedAtMs && frame.capturedAtMs <= after.observedAtMs,
    label + ' original custody changed');
    requireThat(closed(inspection, ['issuer', 'driver', 'inspectedAtMs', 'state',
        ...(confirmation ? ['dialogText', 'confirmLabel', 'confirmCoordinate'] : [])]) && inspection.issuer === '/root' &&
        inspection.driver === '/root/recovery_game_driver' && inspection.state === (confirmation ? 'EXIT_GAME_CONFIRMATION' : 'INTENDED_DISPOSABLE_SEED_PAUSED') &&
        Number.isSafeInteger(inspection.inspectedAtMs) && inspection.inspectedAtMs >= frame.capturedAtMs &&
        inspection.inspectedAtMs <= after.observedAtMs && inspection.inspectedAtMs <= now, label + ' visible inspection required');
    if (confirmation) requireThat(exitConfirmationProfile(binding.exitConfirmation) &&
        inspection.dialogText === binding.exitConfirmation.dialogText && inspection.confirmLabel === binding.exitConfirmation.confirmLabel &&
        Array.isArray(inspection.confirmCoordinate) && inspection.confirmCoordinate.length === 2 &&
        inspection.confirmCoordinate.every((/** @type {any} */ value) => Number.isSafeInteger(value)) &&
        isDeepStrictEqual(inspection.confirmCoordinate, args?.coordinate), 'matching declared exit confirmation required');
    return receipt.sha256;
}

/** Full source/clock/history recheck immediately before every input reservation.
 * @param {any} binding @param {any} args @param {string} [tool] @param {any} [after] */
export function reserveCampaignAction(binding, args, tool = binding.batch === 'E1' ? 'key' : 'left_click', after) {
    const admitted = readCampaignAuthorization(binding.sourcePath, { root: binding.workspaceRoot, runID: binding.runID, input: true });
    requireThat(isDeepStrictEqual(admitted.binding, binding), 'mutation custody changed');
    update(binding, state => {
        const wizardStep = binding.schemaVersion === 6 ? seedWizardInput(binding, state, args, after, Date.now()) : undefined;
        if (state.pending) {
            const previous = state.pending, now = Date.now();
            const resolved = state.resolvedActions ?? [];
            requireThat(Array.isArray(resolved) && resolved.length < 7, 'bounded resolved input history required');
            const completed = resolved.filter((/** @type {any} */ row) => isDeepStrictEqual(owner(row), owner(binding)));
            const activationToPlay = binding.batch === 'B1' && previous.tool === 'activate_window' &&
                tool === 'left_click' && completed.length === 0;
            const seedEntry = binding.batch === 'E1' && previous.tool === 'left_click' &&
                ((completed.length < 2 && tool === 'left_click') || (completed.length === 2 && tool === 'key'));
            const exitConfirmation = binding.batch === 'E2' && previous.tool === 'left_click' && tool === 'left_click' &&
                completed.length === 0 && exitConfirmationProfile(binding.exitConfirmation);
            const generate = binding.schemaVersion === 5 && binding.batch === 'E1' && previous.tool === 'left_click' &&
                tool === 'left_click' && completed.length === 0;
            requireThat(((binding.schemaVersion === 3 && (activationToPlay || seedEntry || exitConfirmation)) ||
                ([4, 5].includes(binding.schemaVersion) && activationToPlay) || generate || wizardStep) && previous.status === 'AWAITING_AFTER' &&
                isDeepStrictEqual(owner(previous), owner(binding)) &&
                closed(after, ['pngBytes', 'window', 'capturedAtMs', 'observedAtMs']) && Buffer.isBuffer(after.pngBytes) &&
                after.pngBytes.length <= 32 * 1024 * 1024 && Number.isSafeInteger(previous.knownResultAtMs) &&
                Number.isSafeInteger(after.capturedAtMs) && after.capturedAtMs >= previous.knownResultAtMs &&
                Number.isSafeInteger(after.observedAtMs) && after.observedAtMs >= after.capturedAtMs && after.observedAtMs <= now &&
                after.window?.windowId === binding.campaign.windowId && after.window.windowId ===
                    (previous.tool === 'activate_window' ? previous.args.window_id : previous.args.target_window_id) &&
                args.target_window_id === after.window.windowId && binding.ownedPids.includes(after.window.pid) &&
                after.window.isFocused === true, 'known original AFTER required before next seeded action');
            const dimensions = completePng(after.pngBytes);
            requireThat(dimensions.width === after.window.bounds?.width && dimensions.height === after.window.bounds?.height,
                'original AFTER dimensions changed');
            const pauseInspectionSha256 = seedEntry && tool === 'key' ? seedSceneInspection(binding, previous, after, now) : undefined;
            const exitConfirmationInspectionSha256 = exitConfirmation ? seedSceneInspection(binding, previous, after, now, args) : undefined;
            const generateInspectionSha256 = generate ? inspectGenerationControl(binding, previous, after, now, args) : undefined;
            state.resolvedActions = [...resolved, { ...previous, ...(pauseInspectionSha256 ? { pauseInspectionSha256 } : {}),
                ...(wizardStep?.inspectionSha256 ? { seedWizardInspectionSha256: wizardStep.inspectionSha256 } : {}),
                ...(exitConfirmationInspectionSha256 ? { exitConfirmationInspectionSha256 } : {}),
                ...(generateInspectionSha256 ? { generateInspectionSha256 } : {}),
                after: { captureReceipt: true, freshness: 'UNKNOWN',
                pngSha256: createHash('sha256').update(after.pngBytes).digest('hex'),
                window: structuredClone(after.window), capturedAtMs: after.capturedAtMs, observedAtMs: after.observedAtMs } }];
            state.pending = null;
        }
        requireThat(isDeepStrictEqual(state.provider, owner(binding)) && !state.pending &&
            state.history.reservedActions < binding.campaign.maxReservedActions, 'pending input or original action ceiling');
        state.history.reservedActions++;
        state.pending = { ...owner(binding), reservationID: binding.controllerSha256 + ':' + state.history.reservedActions,
            tool, args: structuredClone(args), status: 'RESERVED', ...(wizardStep ? {wizardStep} : {}) };
    });
}

/** Exact one-shot finite callback marks dispatch only after current custody checks.
 * @param {any} binding @param {any} args @param {string} [tool] */
export function markCampaignActionSent(binding, args, tool = binding.batch === 'E1' ? 'key' : 'left_click') {
    const admitted = readCampaignAuthorization(binding.sourcePath, { root: binding.workspaceRoot, runID: binding.runID, input: true });
    requireThat(isDeepStrictEqual(admitted.binding, binding), 'consent custody changed');
    update(binding, state => {
        requireThat(state.pending?.status === 'RESERVED' && isDeepStrictEqual(owner(state.pending), owner(binding)) &&
            state.pending.tool === tool && isDeepStrictEqual(state.pending.args, args), 'exact in-flight reservation required');
        state.history.sentActions++; state.pending.status = 'SENT'; state.pending.sentAtMs = Date.now();
    });
}

/** Observations remain possible after a known input; uncertainty is never cleared.
 * @param {any} binding @param {boolean} uncertain */
export function finishCampaignAction(binding, uncertain) {
    update(binding, state => {
        requireThat(state.rootRecordKey === binding.campaign.rootClock.recordKey &&
            state.controllerSha256 === binding.controllerSha256 && isDeepStrictEqual(state.provider, owner(binding)),
        'original reservation owner required');
        if (uncertain) state.uncertain = true;
        else {
            requireThat(state.pending?.status === 'SENT' && isDeepStrictEqual(owner(state.pending), owner(binding)), 'known sent reservation required');
            state.pending.status = 'AWAITING_AFTER';
            state.pending.knownResultAtMs = Date.now();
        }
    }, uncertain ? 'latch' : 'current');
}
