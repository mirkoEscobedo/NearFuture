import { makeOutput, testOutputRoot } from './test-output.mjs';
// @ts-check
import { mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
export const png = readFileSync(new URL('./fixtures/readiness.synthetic.png', import.meta.url));
export const expectedImage = { content: [{ type: 'image', mimeType: 'image/png', data: png.toString('base64') }] };
export const screenshotArgs = { quality: 0, target_window_id: 19, width: 480, show_agent_pointer: false };
/** Synthetic boundary data, never a native or root-authority receipt.
 * @param {string} [prefix] */
export function custodyFixture(prefix = 'custody-intent-') {
    const output = makeOutput(prefix);
    const slot = { id: 'F1', png: resolve(output, 'F1.original.png'), payload: resolve(output, 'F1.original.base64.private.txt'),
        receiptRelative: 'metadata/frame-F1-receipt.private.json', intentPath: resolve(output, 'F1.capture-intent.private.json'), callbackReceipt: resolve(output, 'F1.callback-completion.private.json') };
    const sourcePins = [{ path: fileURLToPath(new URL('./fixtures/readiness.synthetic.png', import.meta.url)),
        sha256: createHash('sha256').update(png).digest('hex') }];
    const binding = { id: 'binding-A', batchId: 'B1', evidenceRoot: output, frames: [slot],
        captureRemainingGateMs: 90000, persistRemainingGateMs: 60000, wholeUiBatchMs: 180000,
        maximumCanonicalBase64Characters: 4194304 };
    const admitted = { workspaceRoot: fileURLToPath(new URL('../../../', import.meta.url)), profileId: 'synthetic-profile', rootRunnerCommandExact: 'SYNTHETIC_ROOT_NOT_RUN',
        sourcePins, bindings: [binding], diagnostics: output };
    const rootClock = { recordKey: 'synthetic-owned-root', startMs: 900000, deadlineMs: 1380000,
        cleanupReserveMs: 30000, callerAdmissionDeadlineMs: 1380000,
        originalCallerAdmissionDeadlineMs: 1380000, producerRecordKey: 'synthetic-controller-read', effectiveDeadlineMs: 1380000 };
    const controllerDeadline = new Date(1380000).toISOString().replace('.000Z', '.0000000Z');
    const intent = { schemaVersion: 1, kind: 'NATIVE99_CAPTURE_INTENT_V1', profileId: admitted.profileId,
        batchId: 'B1', bindingId: binding.id, frameId: 'F1', createdMs: 999950, args: screenshotArgs,
        window: { pid: 101, windowId: 19, bounds: { x: 0, y: 0, width: 480, height: 220 } },
        sourcePins: sourcePins.map(pin => ({ ...pin })), rootClock, batchClock: { startMs: 950000, lastMs: 999900, deadlineMs: 1130000 },
        producerWitness: { producerRecordKey: rootClock.producerRecordKey, controllerPid: 11,
            controllerCreationTime: new Date(901000).toISOString(), controllerKernelCreationFileTime: '134371266963297182',
            conservativeOuterDeadlineUtc: controllerDeadline, controllerDeadlineUtc: controllerDeadline, effectiveDeadlineMs: 1380000 },
        rootCallRecord: { key: rootClock.recordKey, value: { startMs: 900000,
            args: { cmd: admitted.rootRunnerCommandExact }, raw: { session_id: 777 } } },
        ownedWitness: { rootRecordKey: rootClock.recordKey, atMs: 999900,
            source: { startMs: 999800, endMs: 999900, elapsedMs: 100, terminal: { exit_code: 0, output: 'SYNTHETIC_ONLY' } },
            raw: { controller: { controllerPid: 11 }, current: [], lease: { authorized: false, syntheticOnly: true },
                planSHA256: '1'.repeat(64), rootLeaseSHA256: '2'.repeat(64) },
            row: { pid: 101, parentPid: 11, creationTime: new Date(902000).toISOString(),
                kernelCreationFileTime: '134371266963297183', executable: 'SYNTHETIC_NOT_EXECUTABLE',
                sessionId: 1, inSupervisorJob: true, supervisorJobPids: [11, 101],
                mainWindowHandle: '19', mainWindowObservedAtUtc: new Date(999900).toISOString() } } };
    const intentPath = slot.intentPath;
    writeFileSync(intentPath, JSON.stringify(intent) + '\n', { flag: 'wx' });
    let reads = 0;
    return { output, slot, admitted, intent, intentPath,
        readIntent: () => { reads++; return { path: intentPath, bytes: readFileSync(intentPath) }; },
        reads: () => reads, now: () => 1000000 };
}
