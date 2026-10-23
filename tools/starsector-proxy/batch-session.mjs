// @ts-check
import { AsyncLocalStorage } from 'node:async_hooks';
import { validateCampaignBinding, reserveCampaignAction, finishCampaignAction } from './campaign-authority.mjs';
import { completePng } from './direct-frames.mjs';
const observations = new Set(['doctor', 'policy_status', 'list_windows', 'get_window',
    'get_frontmost_app', 'get_display_size', 'list_running_apps', 'screenshot', 'wait']);
const batches = {
    B1: [{ tool: 'activate_window', optional: true }, { tool: 'left_click' }],
    B2: [{ tool: 'activate_window', optional: true }, { tool: 'mouse_move' }, { tool: 'mouse_move' }],
    B3: [{ tool: 'left_click' }, { tool: 'left_click', optional: true }],
};

/** Validate the closed in-memory binding; file/lease custody is checked by the caller too.
 * @param {any} authorization */
export function validateBatchBinding(authorization) {
    if ([2, 3, 4].includes(authorization?.schemaVersion)) { validateCampaignBinding(authorization); return; }
    const keys = ['batch', 'ownedPids', 'planSha256', 'runID', 'scenario', 'schemaVersion'];
    if (!authorization || typeof authorization !== 'object' || Array.isArray(authorization) ||
        Object.keys(authorization).sort().join() !== keys.join() || authorization.schemaVersion !== 1 ||
        typeof authorization.batch !== 'string' || !Object.hasOwn(batches, authorization.batch) ||
        typeof authorization.runID !== 'string' || !/^[A-Za-z0-9_-]+$/.test(authorization.runID) ||
        typeof authorization.scenario !== 'string' || !authorization.scenario ||
        typeof authorization.planSha256 !== 'string' || !/^[a-fA-F0-9]{64}$/.test(authorization.planSha256) ||
        !Array.isArray(authorization.ownedPids) || !authorization.ownedPids.length ||
        authorization.ownedPids.some(/** @param {any} pid */ pid => !Number.isSafeInteger(pid) || pid <= 0) ||
        new Set(authorization.ownedPids).size !== authorization.ownedPids.length) {
        throw new Error('A closed, current caller batch binding is required');
    }
}

/** @param {any} result */
function textJson(result) {
    const texts = result?.content?.filter(/** @param {any} item */ item => item.type === 'text');
    if (texts?.length !== 1) return undefined;
    try { return JSON.parse(texts[0].text); } catch { return undefined; }
}

/** Serializes complete dispatches and limits attempted mutations. It cannot establish
 * visible image preconditions/postconditions; those remain the driver's responsibility.
 * @param {any} session @param {any} authorization
 * @param {(event: Record<string, unknown>) => void} [record] */
export function createBatchSession(session, authorization, record = () => {}) {
    validateBatchBinding(authorization);
    const binding = structuredClone(authorization);
    const campaign = [2, 3, 4].includes(binding.schemaVersion);
    const seededLauncher = [3, 4].includes(binding.schemaVersion) && binding.batch === 'B1';
    const seededEntry = binding.schemaVersion === 3 && binding.batch === 'E1';
    const firstPageEntry = binding.schemaVersion === 4 && binding.batch === 'E1';
    const seededExitConfirmation = binding.schemaVersion === 3 && binding.batch === 'E2' && binding.exitConfirmation !== undefined;
    const escapeBatch = campaign && binding.batch === 'E1';
    const sequence = seededLauncher ? batches.B1 : seededEntry ?
        [{ tool: 'left_click' }, { tool: 'left_click' }, { tool: 'left_click' }, { tool: 'key' }] :
        firstPageEntry ? [{ tool: 'left_click' }] : seededExitConfirmation ? [{ tool: 'left_click' }, { tool: 'left_click', optional: true }] :
        campaign ? [{ tool: escapeBatch ? 'key' : 'left_click', optional: false }]
        : batches[/** @type {keyof typeof batches} */ (binding.batch)];
    let tail = Promise.resolve();
    let position = 0;
    let inputLatched = false;
    let knownSeedPlay = false;
    let doctor = false, core = false, index = 0;
    const requestContext = new AsyncLocalStorage();
    /** @type {{number: number, tool: string} | undefined} */ let activeRequest;
    let requestNumber = 0;
    /** @type {Map<number, {value: any, index: number}>} */
    const windows = new Map();
    /** @type {{value: any, index: number} | undefined} */ let foreground;
    /** @type {{id: number, index: number, bounds: string, pngBytes?: Buffer, capturedAtMs: number} | undefined} */ let capture;
    /** @param {any} value */
    const owned = value => value && binding.ownedPids.includes(value.pid) &&
        (!campaign || value.windowId === binding.campaign.windowId ||
            (seededLauncher && knownSeedPlay && !inputLatched)) &&
        Number.isSafeInteger(value.windowId) && /starsector/i.test(value.title ?? '') &&
        /^(java|starsector)\.exe$/i.test(value.bundleId ?? '') &&
        value.bounds && Number.isSafeInteger(value.bounds.x) && Number.isSafeInteger(value.bounds.y) &&
        Number.isSafeInteger(value.bounds.width) && value.bounds.width > 0 &&
        Number.isSafeInteger(value.bounds.height) && value.bounds.height > 0 &&
        Number.isSafeInteger(value.bounds.x + value.bounds.width) && Number.isSafeInteger(value.bounds.y + value.bounds.height);
    /** @param {string} message @returns {never} */
    const refuse = message => { inputLatched = true; throw new Error('Batch input refused: ' + message); };
    function latchCampaignFailure() {
        if (!campaign) return;
        capture = undefined; foreground = undefined; windows.clear();
        finishCampaignAction(binding, true);
    }
    /** @param {string} tool @param {any} args @param {AbortSignal | undefined} signal
     * @param {any} progress @param {any} context */
    async function perform(tool, args, signal, progress, context) {
        const mutation = !observations.has(tool);
        const callIndex = ++index;
        try {
            if (!args || typeof args !== 'object' || Array.isArray(args)) refuse('tool arguments must be an object');
            if (signal?.aborted) refuse('request cancelled before dispatch');
            if (mutation) {
                const escapeCall = escapeBatch && tool === 'key';
                if (inputLatched) refuse('input authority already latched');
                let next = position;
                while (next < sequence.length && sequence[next].tool !== tool && sequence[next].optional) next++;
                if (next >= sequence.length || sequence[next].tool !== tool) refuse('mutation outside remaining sequence');
                // Reserve before any delegated effect. Failed/uncertain attempts are never refunded.
                position = next + 1;
                if (campaign) {
                    const target = windows.get(args.target_window_id);
                    const after = capture?.pngBytes && capture.id === args.target_window_id && target &&
                        capture.bounds === JSON.stringify(target.value.bounds) && target.value.isFocused === true &&
                        foreground?.value?.pid === target.value.pid && (foreground?.index ?? -1) > capture.index && target.index > capture.index
                        ? { pngBytes: capture.pngBytes, window: target.value, capturedAtMs: capture.capturedAtMs, observedAtMs: Date.now() } : undefined;
                    reserveCampaignAction(binding, args, tool, after);
                }
                const fields = tool === 'activate_window' ? (campaign ? ['window_id', 'timeout_ms'] : ['window_id', 'timeout_ms', 'approval_token'])
                    : escapeCall ? ['text', 'repeat', 'target_window_id', 'target_app', 'focus_strategy']
                    : campaign ? ['target_window_id', 'target_app', 'focus_strategy', 'coordinate']
                    : ['target_window_id', 'target_app', 'focus_strategy', 'coordinate', 'approval_token'];
                if (Object.keys(args).some(key => !fields.includes(key))) refuse('unexpected mutation arguments');
                if (escapeCall && (args.text !== 'escape' || args.repeat !== 1)) refuse('only one exact Escape is qualified');
                if (seededEntry && position === 1 && JSON.stringify(args.coordinate) !==
                    JSON.stringify(binding.b1MenuReceipt.loadCoordinate)) refuse('only the inspected Load Game coordinate is qualified');
                if (firstPageEntry && JSON.stringify(args.coordinate) !== JSON.stringify(binding.b1MenuReceipt.newGameCoordinate)) {
                    refuse('only the inspected New Game coordinate is qualified');
                }
                if (campaign && !escapeBatch && !seededLauncher && !(seededExitConfirmation && position === 2) && (tool !== 'left_click' ||
                    JSON.stringify(args.coordinate) !== JSON.stringify(binding.e1MenuReceipt.exitCoordinate))) refuse('only the inspected ordinary Exit coordinate is qualified');
                const id = tool === 'activate_window' ? args.window_id : args.target_window_id;
                const target = windows.get(id);
                if (!doctor || !core || !target || !capture || capture.id !== id ||
                    capture.bounds !== JSON.stringify(target.value.bounds)) refuse('missing current diagnostics, owned target or original BEFORE receipt');
                if (args.target_app !== undefined && args.target_app !== target.value.bundleId) refuse('conflicting target application');
                if (tool !== 'activate_window' && (args.focus_strategy !== 'strict' ||
                    target.value.isFocused !== true || foreground?.value?.pid !== target.value.pid ||
                    (foreground?.index ?? -1) <= capture.index || target.index <= capture.index)) {
                    refuse('strict fresh focus/window checks required after BEFORE capture');
                }
                if (tool !== 'activate_window' && !escapeCall) {
                    const [x, y] = Array.isArray(args.coordinate) ? args.coordinate : [];
                    const bounds = target.value.bounds;
                    if (!Array.isArray(args.coordinate) || args.coordinate.length !== 2 || !Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
                        x < bounds.x || y < bounds.y || x >= bounds.x + bounds.width || y >= bounds.y + bounds.height) {
                        refuse('pointer coordinate outside the verified owned window');
                    }
                }
                capture = undefined; foreground = undefined;
            } else if (tool === 'screenshot') {
                const target = windows.get(args.target_window_id);
                if (!doctor || !core || !target || args.quality !== 0 || args.width !== target.value.bounds.width ||
                    Object.keys(args).some(key => !['target_window_id', 'target_app', 'quality', 'width', 'provider', 'show_agent_pointer'].includes(key)) ||
                    (args.target_app !== undefined && args.target_app !== target.value.bundleId)) {
                    refuse('only a diagnosed owned target original at native width may be captured');
                }
            }
            record({ phase: 'before-dispatch', index: callIndex, tool, mutation, position, inputLatched });
            const result = await session.dispatch(tool, args, signal, progress, context);
            if (result?.isError || signal?.aborted) inputLatched = true;
            if (!result?.isError && !signal?.aborted) {
                const value = textJson(result);
                if (tool === 'doctor') { doctor = value?.ok === true; if (!doctor) inputLatched = true; }
                if (tool === 'policy_status') { core = value?.profile === 'core'; if (!core) inputLatched = true; }
                if (tool === 'activate_window' && (value?.activated !== true || value?.windowId !== args.window_id)) inputLatched = true;
                if (tool === 'get_frontmost_app') {
                    const app = value?.app;
                    foreground = app && typeof app === 'object' && !Array.isArray(app) &&
                        Number.isSafeInteger(app.pid) && typeof app.bundleId === 'string' && typeof app.displayName === 'string'
                        ? { value: app, index: callIndex } : undefined;
                }
                if (tool === 'get_window') {
                    if (owned(value) && value.windowId === args.window_id) windows.set(value.windowId, { value, index: callIndex });
                    else windows.delete(args.window_id);
                }
                if (tool === 'screenshot') {
                    const images = result?.content?.filter(/** @param {any} item */ item => item.type === 'image');
                    if (images?.length !== 1 || images[0].mimeType !== 'image/png' || !images[0].data) {
                        refuse('original PNG receipt missing');
                    }
                    let pngBytes;
                    if ([3, 4].includes(binding.schemaVersion)) {
                        const encoded = images[0].data;
                        if (typeof encoded !== 'string' || encoded.length > 44 * 1024 * 1024 ||
                            !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) refuse('bounded canonical original PNG required');
                        pngBytes = Buffer.from(encoded, 'base64');
                        if (pngBytes.toString('base64') !== encoded) refuse('canonical original PNG required');
                        const dimensions = completePng(pngBytes), window = windows.get(args.target_window_id)?.value;
                        if (dimensions.width !== window?.bounds.width || dimensions.height !== window?.bounds.height) refuse('original PNG dimensions changed');
                    }
                    capture = { id: args.target_window_id, index: callIndex, pngBytes, capturedAtMs: Date.now(),
                        bounds: JSON.stringify(windows.get(args.target_window_id)?.value.bounds) };
                }
            }
            if (inputLatched) latchCampaignFailure();
            else if (campaign && mutation) {
                finishCampaignAction(binding, false);
                // Observation only: the Play sequence is exhausted. A new provider
                // must separately prove this window's immutable owned handoff.
                if (seededLauncher && tool === 'left_click') knownSeedPlay = true;
            }
            record({ phase: 'after-dispatch', index: callIndex, tool, mutation, position, inputLatched });
            return result;
        } catch (error) {
            inputLatched = true;
            latchCampaignFailure();
            record({ phase: 'refused-or-failed', index: callIndex, tool, mutation, position, inputLatched });
            // The pinned modern registry's elicitation closure suspends before native
            // effects. Do not expose a resumable approval round with invalidated receipts.
            if (error && typeof error === 'object' && 'mcpInputRequired' in error && error.mcpInputRequired === true) {
                throw new Error('UNSUPPORTED_APPROVAL_CONTINUATION: stop and reacquire under fresh authority');
            }
            throw error;
        }
    }
    /** Gate the complete registered handler, not merely its native execution.
     * @param {string} tool @param {() => Promise<any>} operation */
    async function runRequest(tool, operation) {
        if (activeRequest && requestContext.getStore() === activeRequest) return operation();
        if (activeRequest) {
            inputLatched = true;
            latchCampaignFailure();
            record({ phase: 'request-overlap', tool, activeTool: activeRequest.tool,
                activeRequest: activeRequest.number, inputLatched, rejectedBeforeNativeDispatch: true });
            throw new Error('Concurrent Computer Use request refused; input authority latched');
        }
        const request = { number: ++requestNumber, tool };
        try { record({ phase: 'request-start', tool, request: request.number }); }
        catch (error) { inputLatched = true; latchCampaignFailure(); throw error; }
        activeRequest = request;
        return requestContext.run(request, async () => {
            try { return await operation(); }
            catch (error) { inputLatched = true; latchCampaignFailure(); throw error; }
            finally {
                // Clear even if a receipt write fails; the input latch remains closed.
                try { record({ phase: 'request-end', tool, request: request.number, inputLatched }); }
                catch (error) { inputLatched = true; latchCampaignFailure(); throw error; }
                finally { activeRequest = undefined; }
            }
        });
    }
    /** @param {string} tool @param {any} args @param {AbortSignal | undefined} signal
     * @param {any} progress @param {any} context */
    function submit(tool, args, signal, progress, context) {
        const result = tail.then(() => perform(tool, args, signal, progress, context));
        tail = result.then(() => {}, () => {});
        return result;
    }
    return { ...session, runRequest,
        /** @param {string} tool @param {any} args @param {AbortSignal} [signal]
         * @param {any} [progress] @param {any} [context] */
        dispatch(tool, args, signal, progress, context) {
            return runRequest(tool, () => submit(tool, args, signal, progress, context));
        },
        getLastScreenshot: session.getLastScreenshot?.bind(session),
    };
}
