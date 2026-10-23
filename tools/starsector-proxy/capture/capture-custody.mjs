// @ts-check
import { readFileSync, writeFileSync, lstatSync, existsSync } from 'node:fs';
import { resolve, dirname, basename, relative, isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { persistDirectFrame } from '../direct-frames.mjs';
const CLOSED = 'NATIVE99_CAPTURE_CUSTODY_REFUSED';
/** @param {Buffer|string} bytes */ const sha = bytes => createHash('sha256').update(bytes).digest('hex');
/** @param {any} value */ const copy = value => JSON.parse(JSON.stringify(value));
/** @param {boolean} condition @param {string} code @returns {asserts condition} */ function requireThat(condition, code) { if (!condition) throw new Error(code); }
/** @param {string} root @param {string} path */ function inside(root, path) {
    const suffix = relative(resolve(root), resolve(path));
    return !!suffix && !suffix.startsWith('..') && !isAbsolute(suffix);
}
/** No capture output may be redirected through an existing link.
 * @param {string} path */
function ordinary(path) {
    let part = resolve(path);
    while (true) {
        requireThat(!lstatSync(part).isSymbolicLink(), 'ORDINARY_CAPTURE_PATH_REQUIRED');
        const parent = dirname(part); if (parent === part) return; part = parent;
    }
}
/** Existing composition supplies admitted slots and current caller-origin proof.
 * This translates DATA; it does not establish host ownership or grant native authority.
 * @param {{admitted: any, readIntent: () => {path: string, bytes: Buffer}, now?: () => number}} options */
export function createCaptureCustody({ admitted, readIntent, now = Date.now }) {
    const config = copy(admitted), used = new Set();
    const watchdog = config.rootSessionWatchdogMs === undefined ? 480000 : config.rootSessionWatchdogMs;
    requireThat(Number.isSafeInteger(watchdog) && watchdog >= 480000 && watchdog <= 43200000,
        'ADMITTED_SESSION_WATCHDOG_REQUIRED');
    /** @type {WeakMap<object, any>} */ const byArgs = new WeakMap();
    /** @type {any} */ let active = null;
    let failed = false;
    requireThat(typeof readIntent === 'function' && typeof config.workspaceRoot === 'string' && isAbsolute(config.workspaceRoot) &&
        typeof config.profileId === 'string' && config.profileId.length > 0 && typeof config.diagnostics === 'string' &&
        typeof config.rootRunnerCommandExact === 'string' && config.rootRunnerCommandExact.length > 0 &&
        Array.isArray(config.bindings) && config.bindings.length > 0 && Array.isArray(config.sourcePins) && config.sourcePins.length > 0 &&
        config.sourcePins.every((/** @type {any} */ pin) => typeof pin?.path === 'string' && isAbsolute(pin.path) &&
            typeof pin.sha256 === 'string' && /^[a-fA-F0-9]{64}$/.test(pin.sha256)), 'ADMITTED_CAPTURE_BINDINGS_REQUIRED');
    const privateRoot = resolve(config.workspaceRoot, '.tmp');
    requireThat(inside(privateRoot, config.diagnostics), 'PRIVATE_CAPTURE_DIAGNOSTICS_REQUIRED');
    ordinary(config.diagnostics);
    const bindingIds = new Set(), batchIds = new Set();
    for (const binding of config.bindings) {
        requireThat(typeof binding.id === 'string' && binding.id.length > 0 && typeof binding.batchId === 'string' &&
            binding.batchId.length > 0 && !bindingIds.has(binding.id) && !batchIds.has(binding.batchId), 'EXACT_ADMITTED_BINDING_IDENTITY_REQUIRED');
        bindingIds.add(binding.id); batchIds.add(binding.batchId);
        requireThat(inside(privateRoot, binding.evidenceRoot) && Array.isArray(binding.frames) && binding.frames.length <= 6 &&
            binding.captureRemainingGateMs === 90000 && binding.persistRemainingGateMs === 60000 &&
            binding.wholeUiBatchMs === 180000 && binding.maximumCanonicalBase64Characters === 4194304, 'EXISTING_CAPTURE_BOUNDS_REQUIRED');
        ordinary(binding.evidenceRoot);
        for (const slot of binding.frames) {
            const name = basename(slot.png), stem = name.slice(0, -'.original.png'.length);
            requireThat(typeof slot.id === 'string' && /^[A-Za-z0-9][A-Za-z0-9_.:-]{0,511}$/.test(slot.id) &&
                inside(binding.evidenceRoot, slot.png) && /^F[1-9][0-9]{0,2}\.original\.png$/.test(name) &&
                resolve(slot.payload) === resolve(dirname(slot.png), stem + '.original.base64.private.txt') &&
                resolve(slot.intentPath) === resolve(dirname(slot.png), stem + '.capture-intent.private.json') &&
                resolve(slot.callbackReceipt) === resolve(dirname(slot.png), stem + '.callback-completion.private.json'), 'DECLARED_CAPTURE_SLOT_REQUIRED');
            ordinary(dirname(slot.png));
        }
    }
    /** @param {any} error */
    function latch(error) {
        if (!failed) {
            failed = true;
            try {
                writeFileSync(resolve(config.diagnostics, 'raw-capture-custody-failure.private.json'), JSON.stringify({
                    schemaVersion: 1, kind: 'NATIVE99_CAPTURE_CUSTODY_FAILURE_V1', status: 'HOLD', atMs: now(),
                    error: { name: error?.name, message: String(error?.message ?? error), code: error?.code },
                    frozenIntent: active?.intent ?? null, frozenIntentUtf8Base64: active?.intentUtf8Base64 ?? null, intentSha256: active?.intentSha256 ?? null,
                    noFurtherNativeDispatch: true, nativeAcceptance: false,
                }) + '\n', { flag: 'wx' });
            } catch { /* failure remains sticky even when its private diagnostic cannot be written */ }
        }
        return new Error(CLOSED);
    }
    /** Preserve an actual result that cannot qualify, without issuing a completion.
     * @param {any} result @param {string} name */
    function retainUnqualified(result, name) {
        const text = JSON.stringify(result) + '\n';
        requireThat(Buffer.byteLength(text) <= 32 * 1024 * 1024, 'BOUNDED_RAW_CAPTURE_REQUIRED');
        writeFileSync(resolve(config.diagnostics, name), text, { flag: 'wx' });
    }
    /** @param {any} intent @param {any} binding @param {number} n @param {number} gate */
    function clock(intent, binding, n, gate) {
        const root = intent.rootClock, batch = intent.batchClock, producer = intent.producerWitness;
        const safe = Number.isSafeInteger;
        requireThat(root && batch && producer && safe(n) && safe(root.startMs) &&
            root.deadlineMs === root.startMs + watchdog && root.cleanupReserveMs === 30000 &&
            (config.rootSessionWatchdogMs === undefined ||
                (root.sessionWatchdogMs === watchdog && producer.sessionWatchdogMs === watchdog)) &&
            safe(root.callerAdmissionDeadlineMs) && root.callerAdmissionDeadlineMs > root.startMs &&
            root.callerAdmissionDeadlineMs <= root.deadlineMs, 'EXISTING_ROOT_CLOCK_REQUIRED');
        const utc = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{7}Z$/;
        requireThat(utc.test(producer.conservativeOuterDeadlineUtc) && utc.test(producer.controllerDeadlineUtc) &&
            producer.producerRecordKey === root.producerRecordKey && safe(producer.controllerPid) && producer.controllerPid > 0 &&
            typeof producer.controllerCreationTime === 'string' && /^[1-9][0-9]*$/.test(producer.controllerKernelCreationFileTime), 'EXISTING_PRODUCER_WITNESS_REQUIRED');
        const effective = Math.min(root.deadlineMs, root.callerAdmissionDeadlineMs,
            Date.parse(producer.conservativeOuterDeadlineUtc), Date.parse(producer.controllerDeadlineUtc));
        requireThat(safe(effective) && root.effectiveDeadlineMs === effective && producer.effectiveDeadlineMs === effective &&
            safe(batch.startMs) && batch.startMs >= root.startMs && batch.startMs < effective &&
            safe(batch.lastMs) && batch.lastMs >= batch.startMs && batch.lastMs <= n &&
            batch.deadlineMs === Math.min(batch.startMs + binding.wholeUiBatchMs, effective - root.cleanupReserveMs) &&
            n >= batch.startMs && batch.deadlineMs - n >= gate && effective - root.cleanupReserveMs - n >= gate, 'EXISTING_CAPTURE_TIME_GATE');
    }
    /** @param {any} context */
    function authorizeToolCall(context) {
        if (failed) throw new Error(CLOSED);
        if (context.definition.name !== 'screenshot') {
            if (active && context.definition.meta?.mutates === true) throw latch(new Error('INPUT_DURING_CAPTURE'));
            return;
        }
        try {
            requireThat(!active, 'ONE_CAPTURE_AT_A_TIME');
            const source = readIntent(), bytes = Buffer.from(source.bytes);
            requireThat(bytes.length > 0 && bytes.length <= 256 * 1024, 'BOUNDED_CAPTURE_INTENT_REQUIRED');
            const intent = JSON.parse(bytes.toString('utf8'));
            const binding = config.bindings.find((/** @type {any} */ item) => item.id === intent.bindingId && item.batchId === intent.batchId);
            const index = binding?.frames.findIndex((/** @type {any} */ item) => item.id === intent.frameId) ?? -1;
            requireThat(intent.schemaVersion === 1 && intent.kind === 'NATIVE99_CAPTURE_INTENT_V1' &&
                intent.profileId === config.profileId && binding && index >= 0 &&
                isDeepStrictEqual(intent.sourcePins, config.sourcePins), 'EXACT_ADMITTED_CAPTURE_INTENT_REQUIRED');
            const slot = binding.frames[index], output = dirname(slot.png), label = 'capture-' + String(index + 1).padStart(3, '0');
            requireThat(resolve(source.path) === resolve(slot.intentPath), 'EXACT_CAPTURE_INTENT_PATH_REQUIRED');
            ordinary(source.path);
            const n = now();
            clock(intent, binding, n, binding.captureRemainingGateMs);
            const rootRecord = intent.rootCallRecord, root = intent.rootClock, witness = intent.ownedWitness, window = intent.window;
            requireThat(rootRecord?.key === root.recordKey && rootRecord.value?.startMs === root.startMs &&
                rootRecord.value.args?.cmd === config.rootRunnerCommandExact &&
                Number.isSafeInteger(rootRecord.value.raw?.session_id) && rootRecord.value.raw.session_id > 0, 'EXACT_CALLER_ROOT_RECORD_REQUIRED');
            requireThat(Number.isSafeInteger(intent.createdMs) && intent.createdMs >= intent.batchClock.lastMs && intent.createdMs <= n &&
                witness?.rootRecordKey === root.recordKey && Number.isSafeInteger(witness.atMs) &&
                witness.atMs >= root.startMs && witness.atMs <= intent.createdMs &&
                witness.source?.terminal?.exit_code === 0 && !witness.source.terminal.session_id &&
                Number.isSafeInteger(window?.pid) && window.pid > 0 && witness.row?.pid === window.pid &&
                witness.row.inSupervisorJob === true && witness.row.supervisorJobPids?.includes(window.pid) &&
                Number.isSafeInteger(witness.row.sessionId) && witness.row.sessionId > 0, 'CURRENT_CALLER_OWNED_WITNESS_REQUIRED');
            requireThat(Number.isSafeInteger(window.windowId) && window.windowId > 0 && Number.isSafeInteger(window.bounds?.width) &&
                window.bounds.width > 0 && Number.isSafeInteger(window.bounds.height) && window.bounds.height > 0 &&
                isDeepStrictEqual(context.args, intent.args) && intent.args.quality === 0 &&
                intent.args.show_agent_pointer === false && intent.args.target_window_id === window.windowId &&
                intent.args.width === window.bounds.width, 'EXACT_CURRENT_CAPTURE_ARGUMENTS_REQUIRED');
            const key = JSON.stringify([intent.bindingId, intent.frameId]);
            requireThat(!used.has(key), 'CAPTURE_INTENT_CANNOT_BE_REUSED');
            for (const path of [slot.png, slot.payload, slot.callbackReceipt, resolve(output, label + '.raw.private.json'),
                resolve(output, label + '.metadata.private.json')]) requireThat(!existsSync(path), 'FRESH_CAPTURE_ARTIFACTS_REQUIRED');
            used.add(key);
            active = { intent, intentPath: source.path, intentSha256: sha(bytes), intentUtf8Base64: bytes.toString('base64'), binding, slot, label, output,
                startMs: n, lastMs: n, args: copy(context.args) };
            byArgs.set(context.args, active);
        } catch (error) { throw latch(error); }
    }
    /** @param {any} context */
    function onToolResult(context) {
        if (context.definition.name !== 'screenshot') return;
        const current = byArgs.get(context.args);
        try {
            requireThat(current && current === active && isDeepStrictEqual(context.args, current.args), 'EXACT_CAPTURE_RESULT_ASSOCIATION_REQUIRED');
            if (failed) { retainUnqualified(context.result, current.label + '.latched.raw.private.json'); throw new Error(CLOSED); }
            const { intent, binding, slot, output, label } = current;
            const requestID = context.mcpContext?.mcpReq?.id;
            requireThat((typeof requestID === 'string' && requestID.length <= 256) || Number.isSafeInteger(requestID), 'ACTUAL_MCP_REQUEST_ID_REQUIRED');
            try {
                const observed = now();
                requireThat(observed >= current.lastMs, 'MONOTONIC_CAPTURE_CLOCK_REQUIRED');
                clock(intent, binding, observed, binding.persistRemainingGateMs);
                current.lastMs = observed;
            } catch (error) { retainUnqualified(context.result, label + '.late.raw.private.json'); throw error; }
            const image = context.result?.content?.find((/** @type {any} */ item) => item.type === 'image');
            if (typeof image?.data === 'string' && image.data.length > binding.maximumCanonicalBase64Characters) {
                retainUnqualified(context.result, label + '.oversize.raw.private.json'); throw new Error('QUALIFIED_ORIGINAL_CAPACITY_EXCEEDED');
            }
            const saved = persistDirectFrame({ output, label, pngName: basename(slot.png), result: context.result, args: context.args,
                window: intent.window, ownedPids: [intent.ownedWitness.row.pid], provenance: {
                    profileId: intent.profileId, batchId: intent.batchId, bindingId: intent.bindingId, frameId: intent.frameId,
                    intentSha256: current.intentSha256, protocolRequestID: requestID, sourcePins: config.sourcePins,
                    ownershipEvidence: 'CALLER_ORIGIN_CURRENT_WITNESS_NOT_INDEPENDENT_HOST_ATTESTATION' } });
            if (context.result.isError === true) { latch(new Error('NATIVE_CAPTURE_FAILED')); return; }
            requireThat(typeof saved.pngPath === 'string' && resolve(saved.pngPath) === resolve(slot.png), 'ACTUAL_DECLARED_PNG_REQUIRED');
            const original = readFileSync(saved.pngPath), raw = readFileSync(saved.rawPath);
            const metadata = JSON.parse(readFileSync(saved.metadataPath, 'utf8'));
            requireThat(metadata.captureReceipt === true && metadata.freshness === 'UNKNOWN' &&
                metadata.pngSha256 === sha(original) && metadata.rawSha256 === sha(raw) &&
                isDeepStrictEqual(JSON.parse(raw.toString('utf8')), context.result) &&
                original.toString('base64') === image.data, 'EXACT_PERSISTED_ORIGINAL_READBACK_REQUIRED');
            const payloadText = image.data + '\n';
            writeFileSync(slot.payload, payloadText, { flag: 'wx' });
            const payloadReadback = readFileSync(slot.payload);
            requireThat(payloadReadback.toString('utf8') === payloadText &&
                Buffer.from(payloadReadback.toString('utf8').slice(0, -1), 'base64').equals(original), 'EXACT_CANONICAL_PAYLOAD_READBACK_REQUIRED');
            const finish = now(); requireThat(finish >= current.lastMs, 'MONOTONIC_CAPTURE_CLOCK_REQUIRED'); clock(intent, binding, finish, 0);
            const proof = {
                schemaVersion: 1, kind: 'NATIVE99_CAPTURE_CALLBACK_COMPLETION_V1', status: 'PERSISTED_ORIGINAL_RECEIPT_ONLY',
                captureReceipt: true, freshness: 'UNKNOWN', profileId: intent.profileId, batchId: intent.batchId,
                bindingId: intent.bindingId, frameId: intent.frameId, protocolRequestID: requestID,
                intentPath: current.intentPath, intentSha256: current.intentSha256, frozenIntentUtf8Base64: current.intentUtf8Base64,
                rawPath: saved.rawPath, rawSha256: sha(raw), metadataPath: saved.metadataPath, metadataSha256: sha(readFileSync(saved.metadataPath)),
                png: slot.png, bytes: original.length, sha256: sha(original), width: metadata.dimensions.width, height: metadata.dimensions.height,
                payload: slot.payload, canonicalBase64Characters: image.data.length, canonicalBase64SHA256: sha(image.data),
                payloadTextCharacters: payloadText.length, payloadTextSHA256: sha(payloadReadback), originalPayloadRoundtripExact: true,
                sourcePins: config.sourcePins, rootRecordSerializedSHA256: sha(JSON.stringify(intent.rootCallRecord)),
                callerRootClock: intent.rootClock, callerBatchClock: intent.batchClock, callerOwnedWitness: intent.ownedWitness,
                ownershipEvidence: 'CALLER_ORIGIN_CURRENT_WITNESS_NOT_INDEPENDENT_HOST_ATTESTATION', startMs: current.startMs, endMs: finish, timingScope: 'END_MS_SAMPLED_BEFORE_COMPLETION_RECEIPT_WRITE',
            };
            writeFileSync(slot.callbackReceipt, JSON.stringify(proof) + '\n', { flag: 'wx' });
        } catch (error) {
            try {
                const path = current ? resolve(current.output, current.label + '.raw.private.json') : null;
                const exact = JSON.stringify(context.result) + '\n';
                if (!path || !existsSync(path) || readFileSync(path, 'utf8') !== exact)
                    retainUnqualified(context.result, (current?.label ?? 'unexpected-capture') + '.rejected-result.raw.private.json');
            } catch { /* Missing raw retention never awards completion or clears the sticky failure. */ }
            throw latch(error);
        }
        finally { byArgs.delete(context.args); if (active === current) active = null; }
    }
    return { authorizeToolCall, onToolResult };
}
