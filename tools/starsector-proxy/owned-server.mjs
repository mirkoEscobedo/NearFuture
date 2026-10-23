// @ts-check
import { spawn as nativeSpawn } from 'node:child_process';
import { once } from 'node:events';
import { randomBytes } from 'node:crypto';
import { resolve } from 'node:path';

export const qualifiedLocalModel = 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL';

/** @param {boolean} disableThinking @param {boolean} disableParallelToolCalls */
export function localRequestBody(disableThinking, disableParallelToolCalls) {
    return disableThinking || disableParallelToolCalls ? {
        ...(disableThinking ? { enable_thinking: false } : {}),
        ...(disableParallelToolCalls ? { parallel_tool_calls: false } : {}),
    } : null;
}

/** @param {any} listed @returns {'invalid' | 'pending' | 'connected'} */
function coreMcpReadiness(listed) {
    if (!Array.isArray(listed?.data)) return 'invalid';
    const companions = new Set(['chrome-devtools', 'debugMcp', 'dart']);
    const seen = new Set();
    let ownStatus;
    for (const row of listed.data) {
        if (!row || typeof row !== 'object' || Array.isArray(row) ||
            !row.status || typeof row.status !== 'object' || Array.isArray(row.status) ||
            seen.has(row.name)) return 'invalid';
        seen.add(row.name);
        if (row.name === 'computer_use') {
            if (!['pending', 'connected'].includes(row.status.status)) return 'invalid';
            ownStatus = row.status.status;
        } else if (!companions.has(row.name) || row.status.status !== 'disabled') return 'invalid';
    }
    if (!ownStatus) return 'invalid';
    return seen.size === 4 && ownStatus === 'connected' ? 'connected' : 'pending';
}

/** One foreground service, owned by the caller's existing Windows Job.
 * No incumbent service, global registration, persistent DB or saved auth.
 * @param {{executable: string, root: string, model?: string, disableThinking?: boolean, disableParallelToolCalls?: boolean,
 * batchAuthorization?: string, batchRunID?: string, spawn?: typeof nativeSpawn}} options
 */
export async function startOwnedServer({ executable, root, model, disableThinking = false,
    disableParallelToolCalls = false, batchAuthorization, batchRunID, spawn = nativeSpawn }) {
    if ((disableThinking || disableParallelToolCalls) && model !== qualifiedLocalModel) throw new Error('Inference control requires the qualified local model');
    if ((batchAuthorization !== undefined || batchRunID !== undefined) &&
        (!batchAuthorization || !batchRunID || !/^[A-Za-z0-9_-]+$/.test(batchRunID))) throw new Error('Complete caller batch binding required');
    const password = randomBytes(32).toString('hex');
    /** @param {unknown} value */
    const safe = value => String(value).replaceAll(password, '[ephemeral-auth-redacted]')
        .replaceAll(Buffer.from('opencode:' + password).toString('base64'), '[ephemeral-auth-redacted]')
        .replace(/\b(?:Bearer|Basic)\s+[A-Za-z0-9+/_=.-]+/gi, '[authorization-redacted]')
        .replace(/\bsk-[A-Za-z0-9_-]+/g, '[key-redacted]');
    const disabled = Object.fromEntries(['computer_use', 'chrome-devtools', 'debugMcp', 'dart'].map(name =>
        [name, { type: 'local', command: [process.execPath, '--version'], disabled: true }]));
    const body = localRequestBody(disableThinking, disableParallelToolCalls);
    const overlay = { mcp: { servers: disabled }, permissions: [{ action: 'execute', resource: '*', effect: 'deny' }],
        ...(body ? { providers: { 'unsloth-studio': { models: {
            'unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL': { body },
        } } } } : {}) };
    const env = { ...process.env, OPENCODE_AUTO_SHARE: 'false', OPENCODE_DB: ':memory:',
        OPENCODE_PASSWORD: password, OPENCODE_SERVER_PASSWORD: password,
        OPENCODE_CONFIG_CONTENT: JSON.stringify(overlay) };
    const host = spawn(executable, ['serve', '--stdio', '--hostname', '127.0.0.1', '--port', '0'],
        { cwd: root, env, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
    const closed = once(host, 'close');
    // Observe spawn errors immediately while startup obtains its own safe error.
    void closed.catch(() => {});
    host.stdin.on('error', () => {});
    // Consume bounded stderr without persisting potentially sensitive diagnostics.
    let stderrSize = 0;
    host.stderr.on('data', chunk => { stderrSize += chunk.length; if (stderrSize > 65536) host.kill(); });
    let closing = false;
    async function close() {
        if (closing) return;
        closing = true;
        if (!host.stdin.destroyed) host.stdin.end();
        let timer;
        try {
            const done = await Promise.race([closed.then(() => true, () => true),
                new Promise(resolveWait => { timer = setTimeout(() => resolveWait(false), 5000); })]);
            if (!done) { host.kill(); await closed; }
        } finally { clearTimeout(timer); }
    }
    try {
        const url = await new Promise((resolveURL, reject) => {
            let buffer = '';
            const timer = setTimeout(() => reject(new Error('Owned OpenCode startup exceeded 10000ms')), 10000);
            const fail = () => { clearTimeout(timer); reject(new Error('Owned OpenCode startup failed')); };
            host.once('error', fail); host.once('close', fail);
            const receive = /** @param {Buffer} chunk */ chunk => {
                buffer += chunk;
                if (buffer.length > 65536) { fail(); return; }
                const end = buffer.indexOf('\n');
                if (end < 0) return;
                try {
                    const startup = JSON.parse(buffer.slice(0, end));
                    const parsed = new URL(startup.url);
                    if (Object.keys(startup).join() !== 'url' || parsed.protocol !== 'http:' ||
                        parsed.hostname !== '127.0.0.1' || !parsed.port || parsed.username || parsed.password) throw new Error();
                    clearTimeout(timer); host.stdout.off('data', receive); host.stdout.resume(); resolveURL(parsed.origin);
                } catch { fail(); }
            };
            host.stdout.on('data', receive);
        });
        const args = ['--server', /** @type {string} */ (url)];
        /** @param {string} operation @param {string[]} extra @param {number} timeout */
        async function api(operation, extra = [], timeout = 20000) {
            const child = spawn(executable, ['api', ...args, '--log-level', 'none', operation,
                '--param', 'location[directory]=' + root, ...extra],
                { cwd: root, env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
            const terminal = once(child, 'close');
            let timedOut = false;
            const timer = setTimeout(() => { timedOut = true; child.kill(); }, timeout);
            let stdout = '', bytes = 0;
            child.stdout.on('data', chunk => { bytes += chunk.length; if (bytes > 4 * 1024 * 1024) child.kill(); else stdout += chunk; });
            child.stderr.resume();
            try {
                const [status] = await terminal;
                if (timedOut || status !== 0 || bytes > 4 * 1024 * 1024) throw new Error('Owned MCP registration failed: ' + operation);
                return operation === 'experimental.mcp.add' && !stdout.trim() ? undefined : JSON.parse(stdout);
            } finally { clearTimeout(timer); }
        }
        await api('experimental.mcp.add', ['--param', 'server=computer_use', '--data', JSON.stringify({ config: {
            type: 'local', command: [process.execPath, resolve(root, batchAuthorization
                ? 'tools/starsector-proxy/guarded-mcp.mjs'
                : 'tools/computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/dist/server.js')],
            cwd: root, disabled: false, codemode: false,
            timeout: { startup: 20000, catalog: 20000, execution: 20000 },
            environment: { COMPUTER_USE_PROFILE: 'core', COMPUTER_USE_ACTIVE_PROFILE: 'core',
                COMPUTER_USE_AUDIT_LOG: resolve(root, '.tmp/starsector-proxy/computer-use-audit.jsonl').replaceAll('\\', '/'),
                ...(batchAuthorization ? { NF_PROXY_BATCH_AUTHORIZATION: batchAuthorization, NF_PROXY_BATCH_RUN_ID: batchRunID } : {}) },
        } })]);
        const deadline = Date.now() + 20000;
        while (true) {
            const remaining = deadline - Date.now();
            if (remaining <= 0) throw new Error('Owned computer_use MCP is not connected within 20000ms');
            const listed = await api('mcp.list', [], remaining);
            if (Date.now() >= deadline) throw new Error('Owned computer_use MCP is not connected within 20000ms');
            const readiness = coreMcpReadiness(listed);
            if (readiness === 'invalid') {
                const diagnostic = {
                    responseKind: Array.isArray(listed) ? 'array' : listed === null ? 'null' : typeof listed,
                    keys: listed && typeof listed === 'object' ? Object.keys(listed).slice(0, 16).map(safe) : [],
                    rowCount: Array.isArray(listed?.data) ? listed.data.length : null,
                    rows: Array.isArray(listed?.data) ? listed.data.slice(0, 16).map(/** @param {any} item */ item => ({
                        name: typeof item?.name === 'string' ? safe(item.name).slice(0, 128) : null,
                        status: typeof item?.status === 'string' ? safe(item.status).slice(0, 128)
                            : typeof item?.status?.status === 'string' ? safe(item.status.status).slice(0, 128) : null,
                        statusKind: typeof item?.status,
                        errorType: typeof item?.status?.error,
                        error: typeof item?.status?.error === 'string' ? safe(item.status.error).slice(0, 2048) : null,
                    })) : [],
                };
                throw new Error('Owned computer_use MCP is not connected: ' + JSON.stringify(diagnostic));
            }
            if (readiness === 'connected') break;
            await new Promise(resolveWait => setTimeout(resolveWait, Math.min(500, Math.max(0, deadline - Date.now()))));
        }
        return { args, env, close };
    } catch (error) { await close(); throw error; }
}
