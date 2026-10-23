// @ts-check
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { openSync, writeSync, closeSync } from 'node:fs';
import { mkdir, writeFile, readFile, rename } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { StringDecoder } from 'node:string_decoder';
import { screenshotFrames } from './frames.mjs';
import { startOwnedServer, qualifiedLocalModel, localRequestBody } from './owned-server.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from './batch-authorization.mjs';

const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
const { values } = parseArgs({ options: {
    opencode: { type: 'string' }, agent: { type: 'string', default: 'starsector-observer' },
    model: { type: 'string', default: 'opencode-go/glm-5.3-flash' },
    prompt: { type: 'string', default: 'Observe the current Starsector window only and stop.' },
    'run-id': { type: 'string' }, 'session-export': { type: 'string' },
    'wait-for-report': { type: 'boolean', default: false },
    'disable-thinking': { type: 'boolean', default: false },
    'disable-parallel-tool-calls': { type: 'boolean', default: false },
    'check-mcp-only': { type: 'boolean', default: false },
    'batch-authorization': { type: 'string' },
} });
if (!values.opencode || !values['run-id'] || !/^[A-Za-z0-9_-]+$/.test(values['run-id'])) {
    throw new Error('Provide --opencode with the existing executable and a safe --run-id');
}
if (!['starsector-observer', 'starsector-proxy'].includes(values.agent)) throw new Error('Unknown proxy agent');
if (!/^[A-Za-z0-9][A-Za-z0-9._-]*\/[A-Za-z0-9][A-Za-z0-9._:/#-]*$/.test(values.model)) {
    throw new Error('Provide a provider/model identifier');
}
if ((values['disable-thinking'] || values['disable-parallel-tool-calls']) && (values.model !== qualifiedLocalModel || values['session-export'])) {
    throw new Error('Inference control requires a new run of the qualified local model');
}
if (values['check-mcp-only'] && values['session-export']) throw new Error('MCP readiness cannot export a historical session');
if (values['batch-authorization'] && (values.agent !== 'starsector-proxy' || values['session-export'] || values['check-mcp-only'])) {
    throw new Error('Batch authorization requires a new input-agent run');
}
if (values.agent === 'starsector-proxy' && !values['session-export'] && !values['check-mcp-only'] &&
    (!values['batch-authorization'] || !values['wait-for-report'])) {
    throw new Error('Input agent requires an explicit current batch authorization and full report');
}
const batch = values['batch-authorization']
    ? readBatchAuthorization(values['batch-authorization'], { root, runID: values['run-id'] }) : undefined;
const requestedBody = localRequestBody(values['disable-thinking'], values['disable-parallel-tool-calls']);
const output = resolve(root, '.tmp/starsector-proxy', values['run-id']);
await mkdir(output, { recursive: false });
if (batch) claimBatchAuthorization(batch);
/** @type {string | undefined} */
let sessionID = values['session-export'];
/** @type {any[]} */
const parts = [];
/** @type {Record<string, unknown>[]} */
const saved = [];
let adapterComplete = false;
let modelRunComplete = false;
/** @type {Awaited<ReturnType<typeof startOwnedServer>> | undefined} */
let runtime;

/** @param {unknown} session */
async function saveFrames(session) {
    for (const frame of screenshotFrames(session)) {
        const sha256 = createHash('sha256').update(frame.bytes).digest('hex');
        const existing = saved.find(item => item.callID === frame.metadata.callID);
        if (existing) {
            if (existing.sha256 !== sha256) throw new Error('Original screenshot changed during session export');
            continue;
        }
        const path = resolve(output, 'frame-' + String(saved.length + 1).padStart(3, '0') + '.png');
        await writeFile(path, frame.bytes, { flag: 'wx' });
        saved.push({ ...frame.metadata, path, sha256 });
        console.log(JSON.stringify({ status: 'FRAME_RECEIVED', path, sha256, freshness: 'unknown' }));
    }
    await writeFile(resolve(output, 'frames.json.tmp'), JSON.stringify({
        agent: values.agent, requestedModel: values['session-export'] ? null : values.model,
        requestedBody,
        batchBinding: batch ? { ...batch.binding, sourceSha256: batch.sourceSha256 } : null,
        sessionID, adapterComplete, modelRunComplete, frames: saved,
        note: 'Capture receipt is independent of run completion. Event times are not rendered-frame ages.',
    }, null, 2) + '\n');
    await rename(resolve(output, 'frames.json.tmp'), resolve(output, 'frames.json'));
}

/** Stream exact CLI bytes to disk before the supervisor truncates terminal output.
 * @param {string[]} args @param {string} label @param {boolean} events
 */
async function execute(args, label, events) {
    const child = spawn(/** @type {string} */ (values.opencode), args, {
        cwd: root, shell: false, detached: false,
        env: runtime?.env ?? { ...process.env, OPENCODE_AUTO_SHARE: 'false' },
        windowsHide: true,
        stdio: ['ignore', 'pipe', 'pipe'],
    });
    const completed = once(child, 'close');
    /** @param {import('node:stream').Readable} stream @param {string} name @param {boolean} decode */
    async function spool(stream, name, decode) {
        const fd = openSync(resolve(output, name), 'wx');
        const decoder = new StringDecoder('utf8');
        let size = 0;
        let pending = '';
        try {
            for await (const chunk of stream) {
                size += chunk.length;
                if (size > 64 * 1024 * 1024) throw new Error('Proxy output exceeds 64 MiB; supervisor must close the owned tree');
                writeSync(fd, chunk);
                if (!decode) continue;
                pending += decoder.write(chunk);
                let end;
                while ((end = pending.indexOf('\n')) >= 0) {
                    const line = pending.slice(0, end);
                    pending = pending.slice(end + 1);
                    /** @type {any} */
                    let event;
                    try { event = JSON.parse(line); } catch { continue; }
                    if (typeof event.sessionID === 'string') sessionID = event.sessionID;
                    if (event.type === 'tool_use' && event.part?.state?.status === 'completed') {
                        parts.push(event.part);
                        if (event.part.tool === 'computer_use_screenshot') {
                            const observation = { info: { id: sessionID }, messages: [{ parts }] };
                            await saveFrames(observation);
                            if (values.agent === 'starsector-observer' && !values['wait-for-report']) {
                                adapterComplete = true;
                                await saveFrames(observation);
                                const result = JSON.stringify({ status: 'OBSERVATION_CAPTURED', output,
                                    window: saved[0].window, frames: saved.map(frame => frame.path),
                                    modelRunComplete: false, freshness: 'unknown' });
                                await new Promise(resolveWrite => process.stdout.write(result + '\n', resolveWrite));
                                // The native supervisor closes our Job Object on root exit,
                                // terminating only this invocation's OpenCode/MCP descendants.
                                process.exit(0);
                            }
                        }
                    }
                    if (event.type === 'text' && typeof event.part?.text === 'string') console.log(event.part.text);
                    if (event.type === 'error') throw new Error('OpenCode reported an error; inspect the retained events');
                }
            }
            pending += decoder.end();
            if (decode && pending.trim()) throw new Error('Incomplete final OpenCode event');
        } finally { closeSync(fd); }
    }
    await Promise.all([
        spool(child.stdout, label + '.jsonl', events),
        spool(child.stderr, label + '.stderr.txt', false),
    ]);
    const [status, signal] = await completed;
    if (status !== 0) throw new Error('OpenCode exited ' + status + ' (' + signal + '); inspect ' + output);
}

try {
    if (!sessionID) {
        runtime = await startOwnedServer({ executable: values.opencode, root, model: values.model,
            disableThinking: values['disable-thinking'], batchAuthorization: batch?.sourcePath,
            disableParallelToolCalls: values['disable-parallel-tool-calls'],
            batchRunID: batch?.binding.runID });
        if (values['check-mcp-only']) {
            const report = { status: 'MCP_READY', directory: root, modelDispatched: false, agentDispatched: false,
                scenarioToolCallsDispatched: false, requestedModel: values.model,
                requestedBody };
            await writeFile(resolve(output, 'mcp-readiness.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
            console.log(JSON.stringify(report));
        } else {
            await execute(['run', ...runtime.args, '--agent', values.agent, '--model', values.model, '--format', 'json', values.prompt], 'events', true);
            modelRunComplete = true;
        }
    }
    if (!values['check-mcp-only']) {
        if (!sessionID || !/^ses_[A-Za-z0-9_-]+$/.test(sessionID)) throw new Error('No valid OpenCode session ID');
        await execute(['session', 'export', ...(runtime?.args ?? ['--standalone']), sessionID], 'session', false);
        const session = JSON.parse(await readFile(resolve(output, 'session.jsonl'), 'utf8'));
        await saveFrames(session);
        if (!saved.length) throw new Error('No verified targeted Starsector PNG; text alone is insufficient');
        adapterComplete = true;
        await saveFrames(session);
        console.log(JSON.stringify({ status: 'CAPTURE_EXPORTED', output, frames: saved.map(frame => frame.path) }));
    }
} catch (error) {
    console.error(error);
    process.exitCode = 1;
} finally { await runtime?.close(); }
