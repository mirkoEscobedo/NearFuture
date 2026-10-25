// @ts-check
import { spawn as nativeSpawn } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, lstatSync, existsSync, realpathSync, readdirSync } from 'node:fs';
import { resolve, dirname, isAbsolute } from 'node:path';
import { Client, StreamableHTTPClientTransport } from '../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';
import { readFiniteConsentAuthority } from './finite-consent.mjs';
import { persistDirectFrame } from './direct-frames.mjs';
import { fixtureProbeSources, readFixtureBinding } from './fixture-binding.mjs';

export const directToolNames = Object.freeze(['doctor', 'policy_status', 'list_windows', 'get_window', 'get_frontmost_app',
    'get_display_size', 'screenshot', 'wait', 'activate_window', 'left_click', 'mouse_move']);
/** Startup can fail after an owned provider exists. Preserve its actual cleanup
 * outcome instead of losing UNKNOWN when the original startup error is thrown. */
export class OwnedHttpStartupError extends Error {
    /** @param {any} cleanupReceipt */
    constructor(cleanupReceipt) { super('Owned provider startup/readiness failed'); this.name = 'OwnedHttpStartupError'; this.cleanupReceipt = cleanupReceipt ?? null; }
}
export const ownedHttpSources = Object.freeze(['.agentic/project.json', 'tools/starsector-proxy/owned-http-batch.mjs',
    'tools/starsector-proxy/direct-mcp.mjs', 'tools/starsector-proxy/direct-frames.mjs',
    'tools/starsector-proxy/guarded-http-child.mjs',
    'tools/starsector-proxy/guarded-http.mjs', 'tools/starsector-proxy/guarded-mcp.mjs', 'tools/starsector-proxy/finite-consent.mjs',
    'tools/starsector-proxy/batch-authorization.mjs', 'tools/starsector-proxy/menu-launch-grant.mjs', 'tools/starsector-proxy/batch-session.mjs', 'tools/starsector-proxy/campaign-authority.mjs',
    'tools/starsector-proxy/fixture-binding.mjs', 'tools/starsector-proxy/campaign-proof.mjs',
    'tools/starsector-proxy/generation-proof.mjs', 'tools/starsector-proxy/seed-creation-history.mjs', 'tools/starsector-proxy/seed-failed-attempt.mjs', 'tools/starsector-proxy/seed-known-play-attempt.mjs',
    'tools/starsector-proxy/seed-wizard-proof.mjs', 'tools/starsector-proxy/seed-creation-output.mjs',
    'tools/starsector-proxy/seed-creation-custody.mjs', ...fixtureProbeSources]);
/** Explicit complete installed runtime inventory, without an import resolver.
 * @param {string} root @param {string} powershell @param {string} supervisor */
export function requiredOwnedHttpFiles(root, powershell, supervisor) {
    const directory = resolve(root, 'tools/computer-use-mcp/node_modules'); ordinary(directory);
    const runtime = readdirSync(directory, { recursive: true, withFileTypes: true }).flatMap(entry => {
        if (entry.isSymbolicLink()) throw new Error('Ordinary installed runtime required');
        return entry.isFile() && /\.(?:[cm]?js|json|node|exe|dll|wasm|so|dylib)$/i.test(entry.name) ? [resolve(entry.parentPath, entry.name)] : [];
    });
    if (runtime.length === 0) throw new Error('Installed runtime source custody required');
    return [...ownedHttpSources.map(path => resolve(root, path)), ...runtime, realpathSync.native(process.execPath), resolve(supervisor), resolve(powershell)];
}
/** @param {string} path */
function ordinary(path) {
    for (let part = resolve(path);; part = dirname(part)) {
        if (lstatSync(part).isSymbolicLink()) throw new Error('Ordinary provider custody required');
        if (dirname(part) === part) break;
    }
}
/** @param {string} path @param {number} [limit] */
function read(path, limit = 256 * 1024) {
    ordinary(path); const stat = lstatSync(path);
    if (!stat.isFile() || stat.size > limit) throw new Error('Bounded provider receipt required');
    return JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
}
/** @param {string} path @param {any} value */
const write = (path, value) => writeFileSync(path, JSON.stringify(value) + '\n', { flag: 'wx' });
/** @param {string} path */
const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
/** @param {string} manifest @param {string[]} required */
function checkPins(manifest, required) {
    const pins = read(manifest).pins;
    if (!Array.isArray(pins) || pins.length < required.length || pins.length > 1024) throw new Error('Complete provider source custody required');
    const seen = new Set();
    for (const pin of pins) {
        if (typeof pin.path !== 'string' || !isAbsolute(pin.path) || seen.has(resolve(pin.path)) || !/^[a-f0-9]{64}$/i.test(pin.sha256 ?? '')) throw new Error('Closed provider source custody required');
        ordinary(pin.path); seen.add(resolve(pin.path));
        if (sha(pin.path) !== pin.sha256.toLowerCase()) throw new Error('Frozen provider source custody changed');
    }
    if (required.some(path => !seen.has(path))) throw new Error('Required provider source custody missing');
}
/** @param {any} ready @param {any} options */
function validateReady(ready, { runID, admitted, token, providerPid, started, powershell, nodeExecutable, consentMode }) {
    const node = ready?.node, helper = ready?.helper;
    const url = new URL(ready?.url ?? 'invalid:');
    const sessionId = read(resolve(admitted.binding.scenario, 'controller-start.private.json')).sessionId;
    if (ready.schemaVersion !== 1 || ready.runID !== runID || ready.batch !== admitted.binding.batch ||
        ready.sourceSha256 !== admitted.sourceSha256 || ready.bearerSha256 !== createHash('sha256').update(token).digest('hex') ||
        ready.startupTimeoutMs !== 20000 || ready.consentMode !== (consentMode ?? 'decline') ||
        Date.parse(ready.at) < started || Date.parse(ready.at) > Date.now() || !Number.isFinite(Date.parse(ready.at)) ||
        url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port || url.pathname !== '/mcp' || url.search || url.hash || url.username || url.password ||
        !Number.isSafeInteger(node?.pid) || node.pid <= 0 || node.parentPid !== providerPid ||
        resolve(node.executable ?? '') !== nodeExecutable || node.sessionId !== sessionId ||
        !Number.isSafeInteger(helper?.pid) || helper.pid <= 0 || helper.pid === node.pid || helper.parentPid !== node.pid ||
        resolve(helper.executable ?? '') !== resolve(powershell) || helper.sessionId !== sessionId ||
        !Number.isFinite(Date.parse(node.creationTime)) || Date.parse(node.creationTime) < started || Date.parse(node.creationTime) > Date.parse(ready.at) ||
        !Number.isFinite(Date.parse(helper.creationTime)) || Date.parse(helper.creationTime) < Date.parse(node.creationTime) || Date.parse(helper.creationTime) > Date.parse(ready.at) ||
        ready.query !== 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' || !Array.isArray(ready.supervisorJobPids) ||
        ready.supervisorJobPids.some((/** @type {number} */ pid) => !Number.isSafeInteger(pid) || pid <= 0) ||
        new Set(ready.supervisorJobPids).size !== ready.supervisorJobPids.length ||
        !ready.supervisorJobPids.includes(node.pid) || !ready.supervisorJobPids.includes(helper.pid)) throw new Error('Exact fresh owned provider readiness required');
    return url;
}

/** One fresh provider in the existing native verifier's 180-second Windows Job.
 * Only OS spawn is replaceable; admission/claims, real MCP and image custody are not.
 * @param {{root: string, source: string, runID: string, powershell: string, supervisor?: string, consentMode?: string, signal?: AbortSignal}} options
 * @param {{spawn?: typeof nativeSpawn}} [dependencies] */
export async function startOwnedHttpBatch({ root, source, runID, powershell,
    supervisor = resolve(root, 'node_modules/workspace-template-win32-x64/bin/workspace-template.exe'), consentMode, signal }, { spawn = nativeSpawn } = {}) {
    const started = Date.now();
    if (!isAbsolute(powershell) || !isAbsolute(supervisor) || (consentMode !== undefined && consentMode !== 'finite-explicit-consent')) throw new Error('Pinned finite provider configuration required');
    const admitted = readBatchAuthorization(source, { root, runID });
    readFixtureBinding(admitted);
    const campaign = [2, 3, 4, 5, 6].includes(admitted.binding.schemaVersion) ? /** @type {any} */ (admitted.binding).campaign : null;
    const deadline = campaign ? Math.min(started + 180000, campaign.rootClock.effectiveDeadlineMs - campaign.rootClock.cleanupReserveMs) : started + 180000;
    const startupDeadline = Math.min(started + 20000, deadline);
    const campaignMutations = admitted.binding.schemaVersion===6 ? ['left_click'] : [4, 5].includes(admitted.binding.schemaVersion) ? admitted.binding.batch === 'B1' ? ['activate_window', 'left_click'] : ['left_click']
        : admitted.binding.schemaVersion === 3 && admitted.binding.batch === 'B1' ? ['activate_window', 'left_click']
        : admitted.binding.batch === 'E1' ? admitted.binding.schemaVersion === 3 ? ['left_click', 'key'] : ['key'] : ['left_click'];
    const rawToolNames = campaign ? [...directToolNames.filter(name => !['activate_window', 'left_click', 'mouse_move'].includes(name)),
        ...campaignMutations] : directToolNames;
    const nodeExecutable = realpathSync.native(process.execPath);
    const manifest = resolve(admitted.binding.scenario, 'frozen-support-pins.private.json');
    const required = requiredOwnedHttpFiles(root, powershell, supervisor);
    checkPins(manifest, required);
    claimBatchAuthorization(admitted); // Exclusive, consumed before any provider or native effects. Never refunded.
    if (consentMode) readFiniteConsentAuthority({ root, source, runID });
    const output = resolve(root, '.tmp/starsector-proxy', runID);
    ordinary(dirname(output)); mkdirSync(output); ordinary(output);
    const probeRoot = mkdtempSync(resolve(root, '.tmp/starsector-proxy/scenarios/direct-http-' + runID + '-probe-'));
    const project = resolve(output, 'owned-provider'); mkdirSync(project); mkdirSync(resolve(project, '.agentic'));
    const config = read(resolve(root, '.agentic/project.json'));
    const childCommand = { program: nodeExecutable, args: [resolve(root, 'tools/starsector-proxy/guarded-http-child.mjs')], cwd: '.' };
    config.overrides = { workspace: { commands: { 'node:.': [childCommand] } } };
    write(resolve(project, '.agentic/project.json'), config); write(resolve(project, 'package.json'), { name: 'near-future-owned-http-provider', private: true });
    const token = randomBytes(32).toString('hex');
    /** @type {NodeJS.ProcessEnv} */ const env = { ...process.env, NF_PROXY_HTTP_BEARER: token, NF_PROXY_BATCH_AUTHORIZATION: source, NF_PROXY_BATCH_RUN_ID: runID,
        NF_PROXY_HTTP_PROBE_ROOT: probeRoot, NF_PROXY_OWNERSHIP_POWERSHELL: powershell, COMPUTER_USE_PROFILE: 'core',
        COMPUTER_USE_ACTIVE_PROFILE: 'core', COMPUTER_USE_REQUIRE_APPROVAL: 'true', COMPUTER_USE_VISION: 'true',
        COMPUTER_USE_AUDIT_LOG: resolve(output, 'native-audit.private.jsonl') };
    if (process.env.COMPUTER_USE_APPROVAL_TOKEN) throw new Error('Native token shortcuts are unsupported');
    if (consentMode) env.NF_PROXY_HTTP_CONSENT_MODE = consentMode; else delete env.NF_PROXY_HTTP_CONSENT_MODE;
    const provider = spawn(supervisor, ['verify', project, '--scope', 'module', '--module', 'node:.', '--concurrency', '1', '--timeout', '180000', '--json'],
        { cwd: root, env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let stopped = false, overflow = false, spawnFailed = false, killRequested = false;
    function killOwned() { if (!stopped && !killRequested) { killRequested = true; provider.kill(); } }
    const cancelStartup = () => killOwned(); signal?.addEventListener('abort', cancelStartup, { once: true });
    const abort = new AbortController();
    /** @type {Buffer[]} */ const stdout = [];
    /** @type {Buffer[]} */ const stderr = [];
    let size = 0;
    /** @param {Buffer[]} buffer @param {Buffer} chunk */
    function collect(buffer, chunk) { size += chunk.length; if (size > 2 * 1024 * 1024) { overflow = true; killOwned(); } else buffer.push(chunk); }
    provider.stdout?.on('data', chunk => collect(stdout, chunk)); provider.stderr?.on('data', chunk => collect(stderr, chunk));
    provider.once('error', () => { spawnFailed = true; });
    const terminal = new Promise(/** @param {(value: {code: number | null, signal: NodeJS.Signals | null}) => void} done */ done =>
        provider.once('close', (code, signal) => { stopped = true; abort.abort(); done({ code, signal }); }));
    const timer = setTimeout(() => { abort.abort(); killOwned(); }, Math.max(1, deadline - Date.now()));
    timer.unref(); // The real spawned ChildProcess retains ownership while alive.
    /** @type {Client | undefined} */ let client;
    /** @type {StreamableHTTPClientTransport | undefined} */ let transport;
    /** @type {Promise<any> | undefined} */ let closing;
    /** @param {string} value */ const safe = value => value.replaceAll(token, '[ephemeral-bearer-redacted]')
        .replace(/\b(?:Bearer|Basic)\s+[A-Za-z0-9+/_=.-]+/gi, '[authorization-redacted]').replace(/\bsk-[A-Za-z0-9_-]+/g, '[key-redacted]');
    /** @param {Promise<unknown> | undefined} operation @param {number} budget */
    async function boundedClose(operation, budget) {
        let timeout;
        try { await Promise.race([operation, new Promise((done, fail) => {
            timeout = setTimeout(() => fail(new Error('Owned shutdown budget elapsed')), Math.max(1, Math.min(budget, deadline - Date.now())));
        })]); } finally { clearTimeout(timeout); }
    }
    async function close() {
        if (closing) return closing;
        closing = (async () => {
            let closeFailed = false;
            // DELETE terminates the real HTTP session before client transport close.
            try { await boundedClose(transport?.terminateSession(), 2000); } catch { closeFailed = true; }
            try { await boundedClose(client?.close(), 2000); } catch { closeFailed = true; killOwned(); }
            let grace;
            try { await Promise.race([terminal, new Promise(done => { grace = setTimeout(done, Math.max(1, Math.min(3000, deadline - Date.now()))); })]); }
            finally { clearTimeout(grace); }
            killOwned();
            try { await boundedClose(terminal, 2000); } catch { closeFailed = true; }
            const observed = stopped;
            const result = observed ? await terminal : { code: null, signal: null };
            if (observed) clearTimeout(timer);
            const out = safe(Buffer.concat(stdout).toString('utf8')), err = safe(Buffer.concat(stderr).toString('utf8'));
            writeFileSync(resolve(output, 'owned-provider.stdout.private.txt'), out, { flag: 'wx' });
            writeFileSync(resolve(output, 'owned-provider.stderr.private.txt'), err, { flag: 'wx' });
            let verification;
            try { verification = JSON.parse(out); } catch { /* Missing terminal JSON stays failed evidence. */ }
            const steps = verification?.result?.steps?.map((/** @type {any} */ step) => ({ command: step.command, args: step.args, cwd: step.cwd,
                processOwnership: step.processOwnership, status: step.status, timedOut: step.timedOut }));
            // verify reports the raw command-spec cwd, executed relative to its fixed project argument.
            const terminalBoundToChild = steps?.length === 1 && steps[0].command === childCommand.program &&
                JSON.stringify(steps[0].args) === JSON.stringify(childCommand.args) && steps[0].cwd === childCommand.cwd;
            const receipt = { at: new Date().toISOString(), verifierPid: provider.pid, verifierStatus: result.code, signal: result.signal,
                deadline: new Date(deadline).toISOString(), closeFailed, overflow, spawnFailed, terminalObserved: observed, cleanupUnknown: !observed, steps: steps ?? [],
                terminalBoundToChild, verdict: verification?.result?.verdict ?? 'UNKNOWN', terminalReceipt: resolve(output, 'owned-provider-terminal.private.json') };
            write(receipt.terminalReceipt, receipt);
            if (!observed) void terminal.then(late => {
                clearTimeout(timer);
                const lateOut = safe(Buffer.concat(stdout).toString('utf8')), lateErr = safe(Buffer.concat(stderr).toString('utf8'));
                writeFileSync(resolve(output, 'owned-provider-late.stdout.private.txt'), lateOut, { flag: 'wx' });
                writeFileSync(resolve(output, 'owned-provider-late.stderr.private.txt'), lateErr, { flag: 'wx' });
                write(resolve(output, 'owned-provider-late-terminal.private.json'), { at: new Date().toISOString(),
                    verifierPid: provider.pid, verifierStatus: late.code, signal: late.signal, terminalObserved: true,
                    previousUnknownReceipt: receipt.terminalReceipt });
            }).catch(() => {});
            return receipt;
        })(); return closing;
    }
    try {
        const readyPath = resolve(output, 'guarded-http-ready.private.json');
        let ready;
        while (ready === undefined) {
            if (stopped || signal?.aborted || Date.now() >= startupDeadline) throw new Error('Owned provider readiness failed before 20000ms');
            if (existsSync(readyPath)) {
                try { ready = read(readyPath); }
                catch (error) { if (!(error instanceof SyntaxError)) throw error; }
                if (ready !== undefined) break;
            }
            await new Promise(done => setTimeout(done, 25)); // One startup data receipt, never a command channel.
        }
        const url = validateReady(ready, { runID, admitted, token, providerPid: provider.pid, started, powershell, nodeExecutable, consentMode });
        if (stopped || signal?.aborted || Date.now() >= startupDeadline) throw new Error('Owned provider readiness exceeded 20000ms');
        readFixtureBinding(readClaimedBatchAuthorization(source, { root, runID })); checkPins(manifest, required);
        transport = new StreamableHTTPClientTransport(url, { requestInit: { headers: { Authorization: 'Bearer ' + token } },
            reconnectionOptions: { maxRetries: 0, maxReconnectionDelay: 0, initialReconnectionDelay: 0, reconnectionDelayGrowFactor: 1 } });
        client = new Client({ name: 'near-future-finite-driver', version: '1' }, { capabilities: {}, supportedProtocolVersions: ['2025-11-25'] });
        await client.connect(transport, { signal: AbortSignal.any([abort.signal, ...(signal ? [signal] : []), AbortSignal.timeout(Math.max(1, startupDeadline - Date.now()))]) });
        if (Date.now() >= startupDeadline) throw new Error('Owned provider connection exceeded 20000ms');
        const previous = transport.send.bind(transport);
        /** @type {{innerProtocolID: string | number} | undefined} */ let current;
        transport.send = async (message, options) => { if (current && 'method' in message && message.method === 'tools/call' && 'id' in message) current.innerProtocolID = message.id; return previous(message, options); };
        let active = false, calls = 0, captures = 0;
        /** @type {Map<number, any>} */ const windows = new Map();
        /** @type {any[]} */ const actions = [];
        /** @param {string} name @param {any} args @param {{signal?: AbortSignal, outerProtocolID?: string | number, onprogress?: (progress: any) => void}} [options] */
        async function call(name, args, options = {}) {
            if (closing || stopped) throw new Error('Finite batch is closed');
            const callStarted = new Date().toISOString();
            const publicArgs = JSON.parse(JSON.stringify(args, (key, value) => key === 'approval_token' ? '[unsupported-token-redacted]' : value));
            const outerProtocolID = options.outerProtocolID ?? 'UNKNOWN', label = 'call-' + String(++calls).padStart(3, '0');
            const mutating = ['activate_window', 'left_click', 'mouse_move', 'key'].includes(name);
            if (active || !rawToolNames.includes(name)) {
                if (campaign && mutating) windows.clear();
                write(resolve(output, label + '.private.json'), { tool: name, args: publicArgs, outerProtocolID, innerProtocolID: 'UNKNOWN', callStarted,
                    failed: true, reason: 'Overlapping or unsupported raw call; batch retired' });
                await close(); throw new Error('Overlapping or unsupported raw call retired this batch');
            }
            active = true;
            const ids = { innerProtocolID: /** @type {string | number} */ ('UNKNOWN') }; current = ids;
            try {
                if (Object.hasOwn(args, 'approval_token')) throw new Error('Policy token input is unsupported');
                readClaimedBatchAuthorization(source, { root, runID });
                const remaining = Math.min(20000, deadline - Date.now()); if (remaining <= 0) throw new Error('Original provider deadline elapsed');
                // This records only the imminent transport submission. The guarded
                // callback alone reserves/sends root actions; the inner ID is not assigned yet.
                write(resolve(output, label + '.attempt.private.json'), { tool: name, args: publicArgs, outerProtocolID,
                    innerProtocolID: 'UNKNOWN', callStarted, sourceSha256: admitted.sourceSha256, runID, batch: admitted.binding.batch });
                const result = await /** @type {Client} */ (client).callTool({ name, arguments: args }, { signal: AbortSignal.any([abort.signal, ...(options.signal ? [options.signal] : [])]),
                    timeout: remaining, maxTotalTimeout: remaining, resetTimeoutOnProgress: false, onprogress: options.onprogress });
                const raw = JSON.stringify(result) + '\n';
                if (Buffer.byteLength(raw) > 32 * 1024 * 1024 || raw.includes(token)) throw new Error('Bounded nonsecret raw MCP result required');
                writeFileSync(resolve(output, label + '.raw.private.json'), raw, { flag: 'wx' });
                const provenance = { tool: name, args: publicArgs, outerProtocolID, innerProtocolID: ids.innerProtocolID, callStarted,
                    callCompleted: new Date().toISOString(), precedingActions: [...actions], runID, batch: admitted.binding.batch, freshness: 'UNKNOWN' };
                if (name === 'screenshot') persistDirectFrame({ output, label: 'capture-' + String(++captures).padStart(3, '0'), result, args,
                    window: windows.get(args.target_window_id), ownedPids: admitted.binding.ownedPids, provenance });
                write(resolve(output, label + '.private.json'), { ...provenance, isError: result.isError === true });
                if (name === 'get_window') {
                    const texts = result.content.filter(item => item.type === 'text'); let window;
                    try { if (texts.length === 1) window = JSON.parse(texts[0].text); } catch { /* Invalid receipt is never current. */ }
                    if (!result.isError && window?.windowId === args.window_id && admitted.binding.ownedPids.includes(window.pid)) windows.set(window.windowId, window);
                    else windows.delete(args.window_id);
                }
                if (mutating) { actions.push({ ...provenance, isError: result.isError === true }); windows.clear(); }
                if (result.isError) await close();
                return result;
            } catch (error) {
                const path = resolve(output, label + '.private.json');
                if (!existsSync(path)) write(path, { tool: name, args: publicArgs, outerProtocolID, innerProtocolID: ids.innerProtocolID, callStarted,
                    callCompleted: new Date().toISOString(), failed: true, reason: safe(error instanceof Error ? error.message : 'Unknown call failure') });
                await close(); throw error;
            }
            finally { if (campaign && mutating) windows.clear(); active = false; current = undefined; }
        }
        signal?.removeEventListener('abort', cancelStartup); return { output, deadline, call, close };
    } catch {
        let cleanupReceipt;
        try { cleanupReceipt = await close(); } catch { /* No successful receipt may be invented after failed evidence custody. */ }
        throw new OwnedHttpStartupError(cleanupReceipt);
    }
    finally { signal?.removeEventListener('abort', cancelStartup); }
}
