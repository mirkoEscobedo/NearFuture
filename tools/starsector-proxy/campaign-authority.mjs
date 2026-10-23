// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, lstatSync, openSync, closeSync, fsyncSync, renameSync, unlinkSync, realpathSync } from 'node:fs';
import { resolve, dirname, basename, relative, isAbsolute } from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const flow = 'PAUSED_CAMPAIGN_EXIT_NO_SAVE';
/** @param {any} condition @param {string} message @returns {asserts condition} */
function requireThat(condition, message) { if (!condition) throw new Error('Campaign authority refused: ' + message); }
/** @param {string} path @param {number} [limit] */
function file(path, limit = 256 * 1024) {
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
function read(path, limit = 256 * 1024) {
    const found = file(path, limit);
    return { ...found, value: JSON.parse(found.bytes.toString('utf8').replace(/^\uFEFF/, '')) };
}
/** @param {any} value @param {string[]} fields */
function closed(value, fields) {
    return value && typeof value === 'object' && !Array.isArray(value) &&
        Object.keys(value).sort().join() === [...fields].sort().join();
}
/** @param {string} parent @param {string} child */
function inside(parent, child) {
    const suffix = relative(resolve(parent), resolve(child));
    return !!suffix && !suffix.startsWith('..') && !isAbsolute(suffix);
}
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

/** Qualify one known-closed E1 handoff. Past provider time is historical evidence,
 * not a live E2 deadline. No unknown input or shutdown has a transfer path.
 * @param {any} binding @param {any} reference @param {number} now */
function menuHandoff(binding, reference, now) {
    const rawHistory = read(historyPath(binding));
    requireThat(rawHistory.value.uncertain === false, 'uncertain launch input is latched');
    const sourcePath = resolve(binding.scenario, 'E1-authorization.private.json');
    const initialSource = read(sourcePath), claim = read(resolve(binding.scenario, 'guard-E1-consumed.private.json'));
    const initial = claim.value.binding;
    requireThat(rawHistory.value.initialClaimSha256 === claim.sha256 && claim.value.sourcePath === sourcePath &&
        claim.value.sourceSha256 === initialSource.sha256 && initial?.schemaVersion === 2 &&
        initial.planSha256 === initialSource.value.planSha256 && initial.sourceSha256 === initialSource.sha256 && initial.sourcePath === sourcePath && initial.batch === 'E1' &&
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
    const expectedOwner = owner(initial);
    requireThat(action?.status === 'AWAITING_AFTER' && isDeepStrictEqual(owner(action), expectedOwner) &&
        (alreadyTransferred || isDeepStrictEqual(state.provider, expectedOwner)), 'menu reservation proof is unresolved');
    const path = resolve(binding.scenario, 'E1-menu-inspected.private.json');
    requireThat(closed(reference, ['path', 'sha256']) && reference.path === path, 'menu reservation proof reference required');
    const receipt = read(path), value = receipt.value;
    requireThat(receipt.sha256 === reference.sha256 && closed(value, ['schemaVersion', 'kind', 'scenario', 'flow',
        'rootRecordKey', 'controllerSha256', 'e1', 'frame', 'inspection', 'provider']) && value.schemaVersion === 1 &&
        value.kind === 'STARSECTOR_E1_MENU_INSPECTION' && value.scenario === binding.scenario && value.flow === flow &&
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
    const provider = value.provider, output = resolve(binding.workspaceRoot, '.tmp/starsector-proxy', initial.runID);
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
        r.schemaVersion === 1 && r.runID === initial.runID && r.batch === 'E1' && r.sourceSha256 === initialSource.sha256 &&
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
    requireThat(safe(providerClock?.startMs) && providerClock.deadlineMs === Math.min(providerClock.startMs + 180000,
        binding.campaign.rootClock.effectiveDeadlineMs - 30000) && safe(action.sentAtMs) && safe(action.knownResultAtMs) &&
        safe(frame.capturedAtMs) && safe(inspection.inspectedAtMs) && safe(provider.observedClosedAtMs) &&
        action.sentAtMs >= providerClock.startMs && action.knownResultAtMs >= action.sentAtMs &&
        frame.capturedAtMs >= action.knownResultAtMs && closedAt >= frame.capturedAtMs && closedAt <= actualDeadline &&
        actualDeadline <= providerClock.deadlineMs && provider.observedClosedAtMs >= closedAt && provider.observedClosedAtMs <= now &&
        inspection.inspectedAtMs >= frame.capturedAtMs && inspection.inspectedAtMs <= now &&
        Date.parse(r.at) >= providerClock.startMs && Date.parse(r.at) <= action.sentAtMs &&
        Date.parse(r.node.creationTime) >= providerClock.startMs && Date.parse(r.node.creationTime) <= closedAt &&
        Date.parse(r.helper.creationTime) >= Date.parse(r.node.creationTime) && Date.parse(r.helper.creationTime) <= closedAt,
    'known provider closure chronology required');
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
        p.controllerCreationTime === controller.at && /^[1-9][0-9]*$/.test(p.controllerKernelCreationFileTime) &&
        call?.key === r.recordKey && call.value?.startMs === r.startMs &&
        typeof call.value.args?.cmd === 'string' && call.value.args.cmd.length > 0 &&
        safe(call.value.raw?.session_id) && call.value.raw.session_id > 0, 'original effective deadline or producer custody invalid');
}

/** Closed V2 admission for Escape and its known-closed ordinary Exit handoff.
 * @param {string} path @param {{root: string, runID: string, now?: number, input?: boolean}} options */
export function readCampaignAuthorization(path, { root, runID, now = Date.now(), input = false }) {
    const sourcePath = resolve(path), source = read(sourcePath), auth = source.value;
    requireThat(closed(auth, ['batch', 'leasePath', 'leaseSha256', 'ownedReceiptPath', 'ownedReceiptSha256',
        'planPath', 'planSha256', 'scenario', 'schemaVersion']) && auth.schemaVersion === 2 &&
        typeof auth.scenario === 'string' && isAbsolute(auth.scenario) && ['E1', 'E2'].includes(auth.batch) &&
        typeof runID === 'string' && /^[A-Za-z0-9_-]+$/.test(runID), 'closed V2 authorization required');
    const scenario = resolve(auth.scenario);
    requireThat(inside(resolve(root, '.tmp/starsector-proxy/scenarios'), scenario) &&
        sourcePath === resolve(scenario, auth.batch + '-authorization.private.json'), 'exact campaign scenario required');
    /** @type {Record<string, ReturnType<typeof read>>} */ const files = {};
    for (const [key, expected] of Object.entries({ planPath: 'plan.json', leasePath: 'native-lane-lease.private.json',
        ownedReceiptPath: 'driver-' + auth.batch + '-owned.private.json' })) {
        requireThat(typeof auth[key] === 'string' && resolve(auth[key]) === resolve(scenario, expected), 'exact campaign custody path required');
        files[key] = read(auth[key]);
        requireThat(files[key].sha256 === auth[key.replace('Path', 'Sha256')]?.toLowerCase(), 'campaign custody hash changed');
    }
    const plan = files.planPath.value, lease = files.leasePath.value, campaign = plan.campaign;
    requireThat(closed(campaign, ['flow', 'windowId', 'rootClock', 'producerWitness', 'rootCallRecord',
        'historyBaseline', 'maxReservedActions']) && campaign.flow === flow &&
        Number.isSafeInteger(campaign.windowId) && campaign.windowId > 0 &&
        closed(campaign.historyBaseline, ['overallAttempts', 'uiAttempts', 'reservedActions', 'sentActions']) &&
        isDeepStrictEqual(campaign, lease.campaign),
    'closed matching campaign flow required');
    const controllerFile = read(resolve(scenario, 'controller-start.private.json')), controller = controllerFile.value;
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
    const binding = { schemaVersion: 2, batch: auth.batch, runID, scenario, workspaceRoot: resolve(root), planSha256: files.planPath.sha256,
        ownedPids, campaign, sourcePath, sourceSha256: source.sha256, controllerSha256: controllerFile.sha256 };
    let handoff;
    if (auth.batch === 'E2') {
        requireThat(isDeepStrictEqual(plan.e1MenuReceipt, lease.e1MenuReceipt), 'menu reservation proof grant changed');
        handoff = menuHandoff(binding, plan.e1MenuReceipt, now);
        if (handoff.alreadyTransferred) history(binding, now);
    } else history(binding, now);
    requireThat(plan.scenarioId === basename(scenario) && resolve(plan.paths?.scenario ?? '') === scenario &&
        plan.bounds?.maxScenarioActionsIncludingActivations === 8 && plan.bounds?.maxProxyInvocations === 3 &&
        plan.bounds?.proxyTimeoutMs === 180000 && plan.bounds?.ownedGameHardTimeoutMs === campaign.rootClock.sessionWatchdogMs &&
        plan.batches?.some((/** @type {any} */ batch) => batch.id === auth.batch) && lease.schemaVersion === 2 &&
        lease.authorized === true && lease.phase === flow && lease.issuer === '/root' && lease.driver === '/root/recovery_game_driver' &&
        lease.planSha256?.toLowerCase() === files.planPath.sha256 && lease.campaignLoadSaveAllowed === false &&
        lease.ownedGameHardTimeoutMs === campaign.rootClock.sessionWatchdogMs && lease.maxActions === 8 && lease.maxProxyInvocations === 3 &&
        Number.isSafeInteger(campaign.maxReservedActions) &&
        campaign.maxReservedActions === campaign.historyBaseline.reservedActions + lease.maxActions,
    'current finite root campaign lease required');
    requireThat(resolve(lease.supportPinsPath ?? '') === resolve(scenario, 'frozen-support-pins.private.json') &&
        read(lease.supportPinsPath).sha256 === lease.supportPinsSha256?.toLowerCase() &&
        read(resolve(scenario, 'protected-fixture-receipt.private.json')).sha256 === lease.fixtureReceiptSha256?.toLowerCase() &&
        controller.fixtureReceiptSha256?.toLowerCase() === lease.fixtureReceiptSha256?.toLowerCase(), 'original support/fixture custody changed');
    clock(campaign, controller, now, input ? 120000 : 20000);
    const admitted = handoff ? { ...binding, e1MenuReceipt: { path: plan.e1MenuReceipt.path,
        sha256: handoff.receiptSha256, exitCoordinate: handoff.exitCoordinate } } : binding;
    return { sourcePath, sourceSha256: source.sha256, binding: admitted, claimPath: resolve(scenario, 'guard-' + auth.batch + '-consumed.private.json') };
}

/** @param {any} binding */
export function validateCampaignBinding(binding) {
    const fields = ['schemaVersion', 'batch', 'runID', 'scenario', 'workspaceRoot', 'planSha256', 'ownedPids',
        'campaign', 'sourcePath', 'sourceSha256', 'controllerSha256'];
    if (binding?.batch === 'E2') fields.push('e1MenuReceipt');
    requireThat(closed(binding, fields) && binding.schemaVersion === 2 && ['E1', 'E2'].includes(binding.batch) &&
        binding.campaign?.flow === flow && Number.isSafeInteger(binding.campaign.windowId) &&
        typeof binding.workspaceRoot === 'string' && isAbsolute(binding.workspaceRoot) &&
        typeof binding.scenario === 'string' && inside(resolve(binding.workspaceRoot, '.tmp/starsector-proxy/scenarios'), binding.scenario) &&
        binding.sourcePath === resolve(binding.scenario, binding.batch + '-authorization.private.json') &&
        typeof binding.runID === 'string' && /^[A-Za-z0-9_-]+$/.test(binding.runID) &&
        [binding.planSha256, binding.sourceSha256, binding.controllerSha256].every(hash => typeof hash === 'string' && /^[a-fA-F0-9]{64}$/.test(hash)) &&
        Array.isArray(binding.ownedPids) && binding.ownedPids.length > 0 &&
        binding.ownedPids.every((/** @type {any} */ pid) => Number.isSafeInteger(pid) && pid > 0) &&
        new Set(binding.ownedPids).size === binding.ownedPids.length &&
        (binding.batch !== 'E2' || closed(binding.e1MenuReceipt, ['path', 'sha256', 'exitCoordinate'])), 'closed campaign binding required');
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
    if (binding.batch === 'E2') {
        update(binding, state => {
            const proof = menuHandoff(binding, { path: binding.e1MenuReceipt.path, sha256: binding.e1MenuReceipt.sha256 }, Date.now());
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
        state.initialClaimSha256 = read(resolve(binding.scenario, 'guard-E1-consumed.private.json')).sha256;
        state.providerClock = { startMs, deadlineMs: Math.min(startMs + 180000, root.effectiveDeadlineMs - root.cleanupReserveMs) };
    });
}

/** Full source/clock/history recheck immediately before every input reservation.
 * @param {any} binding @param {any} args */
export function reserveCampaignAction(binding, args) {
    const admitted = readCampaignAuthorization(binding.sourcePath, { root: binding.workspaceRoot, runID: binding.runID, input: true });
    requireThat(isDeepStrictEqual(admitted.binding, binding), 'mutation custody changed');
    update(binding, state => {
        requireThat(isDeepStrictEqual(state.provider, owner(binding)) && !state.pending &&
            state.history.reservedActions < binding.campaign.maxReservedActions, 'pending input or original action ceiling');
        state.history.reservedActions++;
        state.pending = { ...owner(binding), reservationID: binding.controllerSha256 + ':' + state.history.reservedActions,
            tool: binding.batch === 'E1' ? 'key' : 'left_click', args: structuredClone(args), status: 'RESERVED' };
    });
}

/** Exact one-shot finite callback marks dispatch only after current custody checks.
 * @param {any} binding @param {any} args */
export function markCampaignActionSent(binding, args) {
    const admitted = readCampaignAuthorization(binding.sourcePath, { root: binding.workspaceRoot, runID: binding.runID, input: true });
    requireThat(isDeepStrictEqual(admitted.binding, binding), 'consent custody changed');
    update(binding, state => {
        requireThat(state.pending?.status === 'RESERVED' && isDeepStrictEqual(owner(state.pending), owner(binding)) &&
            isDeepStrictEqual(state.pending.args, args), 'exact in-flight reservation required');
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
