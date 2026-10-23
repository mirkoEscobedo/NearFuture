// @ts-check
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { crc32, deflateSync } from 'node:zlib';
import { resolve, dirname, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ownedHttpSources, requiredOwnedHttpFiles } from './owned-http-batch.mjs';
import { readBatchAuthorization } from './batch-authorization.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';
import { createFiniteConsentMcp } from './finite-consent.mjs';
import { serveGuardedHttp } from './guarded-http.mjs';
import { syntheticFixtureBinding } from './fixture-test-binding.mjs';
import { syntheticMenuLaunch } from './menu-launch-test-grant.mjs';
const base = resolve(fileURLToPath(new URL('../../.tmp', import.meta.url)));
export const syntheticDoctor = { ok: true, summary: { passed: 1, warned: 0, failed: 0, skipped: 0 },
    platform: { os: 'win32', arch: 'x64', node: process.version }, checks: [] };
/** Standards-generated 1x1 RGBA test fixture, never a transformed native image.
 * @param {string} type @param {Buffer} data */
function pngChunk(type, data) { const out = Buffer.alloc(data.length + 12); out.writeUInt32BE(data.length); out.write(type, 4, 'ascii');
    data.copy(out, 8); out.writeUInt32BE(crc32(out.subarray(4, out.length - 4)), out.length - 4); return out; }
export const syntheticPng = Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'),
    pngChunk('IHDR', Buffer.from('00000001000000010806000000', 'hex')), pngChunk('IDAT', deflateSync(Buffer.alloc(5))), pngChunk('IEND', Buffer.alloc(0))]).toString('base64');
/** @param {any} data */ const json = data => ({ content: [{ type: 'text', text: JSON.stringify(data) }], structuredContent: data });
/** @param {string} tool */
export async function syntheticDispatch(tool) {
    const window = { pid: 101, windowId: 19, title: 'Starsector Synthetic', bundleId: 'java.exe', isFocused: true,
        bounds: { x: 10, y: 10, width: 1, height: 1 } };
    if (tool === 'doctor') return json(syntheticDoctor);
    if (tool === 'policy_status') return json({ profile: 'core', approval_required_for_all: true, approval_token_configured: false,
        destructive_requires_approval: true, audit: { enabled: true, path: 'synthetic' }, allowed_apps: [], blocked_apps: [], sensitive_apps: [], require_approval_for: [] });
    if (tool === 'get_window') return json(window);
    if (tool === 'get_frontmost_app') return json({ app: { pid: 101, bundleId: 'java.exe', displayName: 'Synthetic' } });
    if (tool === 'list_windows') return json({ windows: [window] });
    if (tool === 'screenshot') return { content: [{ type: 'image', mimeType: 'image/png', data: syntheticPng }] };
    return json({ ok: true });
}
/** @param {string} path @param {any} value */
const write = (path, value) => writeFileSync(path, JSON.stringify(value) + '\n');
/** @param {string} path */ const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
/** Real admission, guard and HTTP SDK; only the process/native boundaries are replaced.
 * @param {{wrongParent?: boolean, neverReady?: boolean, terminalNever?: boolean, killReturnsFalse?: boolean, terminalStatus?: number, terminalDelayMs?: number, spoolSecret?: boolean,
 * terminalStep?: {cwd?: string, command?: string, args?: string[]},
 * readyPatch?: (ready: any) => void, dispatch?: (tool: string, args: any, signal?: AbortSignal, progress?: any) => Promise<any>,
 * authority?: Parameters<typeof import('./owned-http-batch.mjs').startOwnedHttpBatch>[0],
 * sessionFactory?: NonNullable<Parameters<typeof createFiniteConsentMcp>[1]>['sessionFactory']}} [changes] */
export function directFixture(changes = {}) {
    mkdirSync(base, { recursive: true }); const root = mkdtempSync(resolve(base, 'synthetic-direct-provider-'));
    const scenario = resolve(root, '.tmp/starsector-proxy/scenarios/fixture'); mkdirSync(scenario, { recursive: true });
    const source = resolve(scenario, 'B1-authorization.private.json'), runID = 'synthetic-direct-run';
    const powershell = resolve(root, 'synthetic-pwsh.exe'), supervisor = resolve(root, 'synthetic-verifier.exe');
    const nodeExecutable = realpathSync.native(process.execPath);
    for (const path of [...ownedHttpSources.map(path => resolve(root, path)), powershell, supervisor]) {
        mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, 'synthetic frozen external source');
    }
    write(resolve(root, '.agentic/project.json'), { version: 2, overrides: {} });
    const runtime = resolve(root, 'tools/computer-use-mcp/node_modules/synthetic-runtime/index.mjs');
    mkdirSync(dirname(runtime), { recursive: true }); writeFileSync(runtime, 'synthetic external runtime');
    const paths = { planPath: resolve(scenario, 'plan.json'), leasePath: resolve(scenario, 'native-lane-lease.private.json'),
        ownedReceiptPath: resolve(scenario, 'driver-B1-owned.private.json') };
    write(paths.planPath, { scenarioId: 'fixture', paths: { scenario }, fixtureBinding: syntheticFixtureBinding(scenario), batches: [{ id: 'B1' }, { id: 'B2' }], bounds: {
        maxScenarioActionsIncludingActivations: 8, maxProxyInvocations: 3, proxyTimeoutMs: 180000, ownedGameHardTimeoutMs: 480000 } });
    write(resolve(scenario, 'protected-fixture-receipt.private.json'), { synthetic: true });
    const pins = requiredOwnedHttpFiles(root, powershell, supervisor);
    write(resolve(scenario, 'frozen-support-pins.private.json'), { pins: pins.map(path => ({ path, sha256: sha(path) })) });
    const fixtureSha = sha(resolve(scenario, 'protected-fixture-receipt.private.json')), now = Date.now();
    write(paths.leasePath, { schemaVersion: 1, authorized: true, phase: 'MENU_SMOKE_ONLY', issuer: '/root', driver: '/root/recovery_game_driver',
        planSha256: sha(paths.planPath), ownedGameHardTimeoutMs: 480000, maxActions: 8, maxProxyInvocations: 3, campaignLoadSaveAllowed: false,
        supportPinsPath: resolve(scenario, 'frozen-support-pins.private.json'), supportPinsSha256: sha(resolve(scenario, 'frozen-support-pins.private.json')), fixtureReceiptSha256: fixtureSha });
    write(resolve(scenario, 'controller-start.private.json'), { at: new Date(now - 2000).toISOString(), controllerPid: 100, sessionId: 1,
        supervisorJobPids: [100, 101], hardTimeoutMs: 480000, installation: resolve(scenario, 'installation'), fixtureReceiptSha256: fixtureSha });
    write(paths.ownedReceiptPath, { processes: [{ pid: 101, inSupervisorJob: true, supervisorJobPids: [100, 101], sessionId: 1,
        creationTime: new Date(now - 1000).toISOString(), executable: resolve(scenario, 'installation/java.exe') }] });
    syntheticMenuLaunch(root, scenario);
    write(source, { schemaVersion: 1, scenario, batch: 'B1', ...paths,
        planSha256: sha(paths.planPath), leaseSha256: sha(paths.leasePath), ownedReceiptSha256: sha(paths.ownedReceiptPath) });
    const nextSource = resolve(scenario, 'B2-authorization.private.json'), nextOwned = resolve(scenario, 'driver-B2-owned.private.json');
    writeFileSync(nextOwned, readFileSync(paths.ownedReceiptPath));
    write(nextSource, { ...JSON.parse(readFileSync(source, 'utf8')), batch: 'B2', ownedReceiptPath: nextOwned, ownedReceiptSha256: sha(nextOwned) });
    const authority = changes.authority ?? { root, source, runID, powershell, supervisor };
    let starts = 0, kills = 0, token = '';
    /** @type {string[]} */ let spawnArgs = [];
    /** @type {Awaited<ReturnType<typeof serveGuardedHttp>> | undefined} */ let host;
    /** @type {Awaited<ReturnType<typeof serveGuardedHttp>>[]} */ const hosts = [];
    /** @type {Promise<void> | undefined} */ let startup;
    const spawn = /** @type {typeof import('node:child_process').spawn} */ (/** @type {any} */ ((/** @type {string} */ program, /** @type {string[]} */ args, /** @type {any} */ options) => {
        starts++; spawnArgs = args; token = options.env.NF_PROXY_HTTP_BEARER;
        const child = /** @type {any} */ (new EventEmitter()); child.pid = 771; child.stdout = new PassThrough(); child.stderr = new PassThrough();
        let ended = false;
        const end = (/** @type {number} */ status) => {
            if (changes.terminalNever) return;
            if (ended) return; ended = true;
            if (changes.spoolSecret) { child.stderr.write(token.slice(0, 17)); child.stderr.write(token.slice(17)); }
            child.stdout.write(JSON.stringify({ schemaVersion: 1, ok: true, command: 'verify', ...(changes.spoolSecret ? { diagnostic: token } : {}), result: { verdict: status === 0 ? 'PASS' : 'FAIL', steps: [{
                command: nodeExecutable, args: [resolve(authority.root, 'tools/starsector-proxy/guarded-http-child.mjs')], cwd: '.',
                processOwnership: 'windows-job-object', status, timedOut: false, ...changes.terminalStep }] } }));
            child.emit('close', status, null);
        };
        child.kill = () => { kills++; if (changes.killReturnsFalse) return false; void startup?.finally(async () => { await host?.close(); end(1); }); return true; };
        startup = (async () => {
            const activeRunID = options.env.NF_PROXY_BATCH_RUN_ID;
            const providerAuthority = { root: authority.root, source: options.env.NF_PROXY_BATCH_AUTHORIZATION, runID: activeRunID };
            const admitted = readBatchAuthorization(providerAuthority.source, { root: authority.root, runID: activeRunID });
            const embedded = options.env.NF_PROXY_HTTP_CONSENT_MODE === 'finite-explicit-consent' ?
                createFiniteConsentMcp(providerAuthority, { sessionFactory: changes.sessionFactory }) : createGuardedMcp(admitted.binding, { session: {
                dispatch: changes.dispatch ?? syntheticDispatch,
            } });
            host = await serveGuardedHttp({ embedded, token });
            hosts.push(host);
            const stamp = new Date().toISOString();
            const ready = { schemaVersion: 1, at: stamp,
                runID: activeRunID, batch: admitted.binding.batch, sourceSha256: admitted.sourceSha256, bearerSha256: createHash('sha256').update(token).digest('hex'),
                startupTimeoutMs: 20000, consentMode: options.env.NF_PROXY_HTTP_CONSENT_MODE ?? 'decline', url: host.url,
                node: { pid: 772, parentPid: changes.wrongParent ? 1 : child.pid, creationTime: stamp, executable: nodeExecutable, sessionId: 1 },
                helper: { pid: 773, parentPid: 772, creationTime: stamp, executable: authority.powershell, sessionId: 1 },
                supervisorJobPids: [772, 773], query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' };
            changes.readyPatch?.(ready);
            if (!changes.neverReady) write(resolve(authority.root, '.tmp/starsector-proxy', activeRunID, 'guarded-http-ready.private.json'), ready);
            void host.closed.then(async () => { if (changes.terminalDelayMs) await new Promise(done => setTimeout(done, changes.terminalDelayMs)); end(changes.terminalStatus ?? 0); }, () => end(1));
        })(); void startup.catch(() => end(1));
        return child;
    }));
    return { scenario, options: authority, spawn,
        next: { source: nextSource, runID: runID + '-next' },
        starts: () => starts, kills: () => kills, token: () => token, args: () => spawnArgs,
        async close() { await startup; await Promise.all(hosts.map(value => value.close())); const suffix = relative(base, root);
            assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix)); rmSync(root, { recursive: true }); } };
}
