// @ts-check
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { openSync, writeSync, closeSync, readFileSync, writeFileSync, lstatSync, readdirSync, existsSync } from 'node:fs';
import { resolve, dirname, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readClaimedBatchAuthorization } from './batch-authorization.mjs';
import { createGuardedMcp } from './guarded-mcp.mjs';
import { serveGuardedHttp } from './guarded-http.mjs';
import { createFiniteConsentMcp, readFiniteConsentAuthority } from './finite-consent.mjs';
import { fixtureProbeSources, readFixtureBinding } from './fixture-binding.mjs';

const probeFiles = ['tools/starsector-proxy/fixture-binding.mjs', ...fixtureProbeSources];
/** @param {string} path */
function ordinary(path) {
    for (let part = resolve(path);; part = dirname(part)) {
        if (lstatSync(part).isSymbolicLink()) throw new Error('Ordinary child paths required');
        if (dirname(part) === part) break;
    }
}
/** @param {string} path */
function read(path) {
    ordinary(path);
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.size > 256 * 1024) throw new Error('Bounded child receipt required');
    return JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
}
/** @param {string} path */
const digest = path => createHash('sha256').update(readFileSync(path)).digest('hex');
/** @param {string} manifest @param {string[]} paths */
function requirePins(manifest, paths) {
    const pins = read(manifest).pins;
    for (const path of paths) {
        ordinary(path);
        const matches = Array.isArray(pins) ? pins.filter(pin => typeof pin.path === 'string' && resolve(pin.path) === path) : [];
        if (matches.length !== 1 || matches[0].sha256?.toLowerCase() !== digest(path)) throw new Error('Qualified ownership probe source custody required');
    }
}
/** Query only; this process is already owned by the caller's existing Job.
 * @param {{root: string, probeRoot: string, powershell?: string, fixtureBinding: {path: string, sha256: string}, signal: AbortSignal}} options */
async function probeOwnership({ root, probeRoot, powershell, fixtureBinding, signal }) {
    if (!powershell || !isAbsolute(powershell)) throw new Error('Pinned PowerShell executable required');
    const env = { ...process.env }; delete env.NF_PROXY_HTTP_BEARER;
    const helper = spawn(powershell, ['-NoLogo', '-NoProfile', '-NonInteractive', '-File',
        resolve(root, 'tools/starsector-proxy/guarded-http-ownership.ps1'), '-EvidenceRoot', probeRoot, '-NodePid', String(process.pid),
        '-BindingPath', fixtureBinding.path, '-BindingSha256', fixtureBinding.sha256],
    { cwd: root, env, windowsHide: true, stdio: 'ignore', signal });
    let failed = false;
    helper.once('error', () => { failed = true; });
    const closed = new Promise(done => helper.once('close', status => done(status)));
    try {
        const status = await closed;
        if (failed || status !== 0 || signal.aborted) throw new Error('Owned ownership probe failed');
        const identity = read(resolve(probeRoot, 'process-identities.private.json'));
        const job = read(resolve(probeRoot, 'job-ownership-probe.private.json'));
        const node = identity.find(/** @param {any} row */ row => row.pid === process.pid);
        const probe = identity.find(/** @param {any} row */ row => row.pid === helper.pid);
        if (identity.length !== 2 || !node || !probe || job.controllerPid !== helper.pid || job.complete !== true ||
            job.nativeLaunched !== false || job.query !== 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' ||
            !Array.isArray(job.currentSupervisorJobPids)) throw new Error('Complete inherited Job ownership required');
        return { node, helper: probe, supervisorJobPids: job.currentSupervisorJobPids, query: job.query };
    } finally {
        if (helper.exitCode === null && helper.signalCode === null) { helper.kill(); await closed.catch(() => {}); }
    }
}

/** One existing claimed batch, one guard and one authenticated HTTP session.
 * The caller retains the ORIGINAL 180-second Job. Readiness after 20 seconds is
 * refused; the cooperative OS probe is signalled then and reaped. Other setup
 * and cleanup remain externally bounded. This child never creates a Job,
 * claims authority or extends time.
 * Only native session creation and the query-only OS probe are injectable.
 * @param {{root: string, source: string, runID: string, token: string, probeRoot: string, powershell?: string, consentMode?: string}} options
 * @param {{sessionFactory?: NonNullable<Parameters<typeof createGuardedMcp>[1]>['sessionFactory'],
 * probe?: (options: {root: string, probeRoot: string, powershell?: string, fixtureBinding: {path: string, sha256: string}, signal: AbortSignal}) => Promise<any>}} [dependencies] */
export async function startGuardedHttpChild({ root, source, runID, token, probeRoot, powershell, consentMode }, dependencies = {}) {
    const started = Date.now(), deadline = started + 20000;
    if (!/^[a-f0-9]{64}$/.test(token) || !/^[A-Za-z0-9_-]+$/.test(runID)) throw new Error('Complete finite child binding required');
    if (consentMode !== undefined && consentMode !== 'finite-explicit-consent') throw new Error('Unknown finite consent mode');
    const admitted = readClaimedBatchAuthorization(source, { root, runID });
    const fixtureBinding = readFixtureBinding(admitted);
    if (consentMode) readFiniteConsentAuthority({ root, source, runID });
    const output = resolve(root, '.tmp/starsector-proxy', runID);
    ordinary(output); ordinary(probeRoot);
    if (dirname(resolve(probeRoot)) !== resolve(root, '.tmp/starsector-proxy/scenarios') || readdirSync(probeRoot).length !== 0)
        throw new Error('Fresh unused caller-owned probe scenario required');
    const readyPath = resolve(output, 'guarded-http-ready.private.json');
    if (existsSync(readyPath)) throw new Error('Preserve existing child readiness');
    const manifest = resolve(admitted.binding.scenario, 'frozen-support-pins.private.json');
    requirePins(manifest, [...probeFiles.map(path => resolve(root, path)), ...(powershell ? [resolve(powershell)] : [])]);
    const fd = openSync(resolve(output, 'batch-dispatch.private.jsonl'), 'wx');
    let logClosed = false;
    function closeLog() { if (!logClosed) { logClosed = true; closeSync(fd); } }
    /** @type {Awaited<ReturnType<typeof serveGuardedHttp>> | undefined} */ let host;
    /** @type {ReturnType<typeof createGuardedMcp> | undefined} */ let embedded;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), Math.max(1, deadline - Date.now()));
    function checkTime() { if (controller.signal.aborted || Date.now() >= deadline) throw new Error('Owned child startup exceeded 20000ms'); }
    try {
        const ownership = await (dependencies.probe ?? probeOwnership)({ root, probeRoot, powershell, fixtureBinding, signal: controller.signal });
        checkTime();
        const node = ownership.node, helper = ownership.helper;
        if (node?.pid !== process.pid || node.parentPid !== process.ppid || resolve(node.executable ?? '') !== resolve(process.execPath) ||
            helper?.parentPid !== process.pid || !Number.isSafeInteger(helper.pid) || helper.pid <= 0 ||
            !Number.isSafeInteger(node.sessionId) || node.sessionId <= 0 || helper.sessionId !== node.sessionId ||
            (powershell && resolve(helper.executable ?? '') !== resolve(powershell)) ||
            !Number.isFinite(Date.parse(node.creationTime)) || Date.parse(node.creationTime) > started ||
            Math.abs(Date.parse(node.creationTime) - (Date.now() - process.uptime() * 1000)) > 2000 ||
            !Number.isFinite(Date.parse(helper.creationTime)) ||
            Date.parse(helper.creationTime) < started || Date.parse(helper.creationTime) > Date.now() ||
            ownership.query !== 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' ||
            !Array.isArray(ownership.supervisorJobPids) ||
            ownership.supervisorJobPids.some((/** @type {number} */ pid) => !Number.isSafeInteger(pid) || pid <= 0) ||
            new Set(ownership.supervisorJobPids).size !== ownership.supervisorJobPids.length ||
            !ownership.supervisorJobPids.includes(process.pid) || !ownership.supervisorJobPids.includes(helper.pid))
            throw new Error('Current exact child and inherited Job identity required');
        // Probe completion cannot outlive the original lease or changed files.
        readFixtureBinding(readClaimedBatchAuthorization(source, { root, runID }));
        requirePins(manifest, [...probeFiles.map(path => resolve(root, path)), ...(powershell ? [resolve(powershell)] : [])]);
        /** @param {Record<string, unknown>} event */
        function record(event) {
            if (event.phase === 'before-dispatch' && event.mutation) readClaimedBatchAuthorization(source, { root, runID });
            writeSync(fd, JSON.stringify({ at: new Date().toISOString(), runID, batch: admitted.binding.batch, ...event }) + '\n');
        }
        const options = { record, sessionFactory: dependencies.sessionFactory };
        embedded = consentMode ? createFiniteConsentMcp({ root, source, runID }, options)
            : createGuardedMcp(admitted.binding, options);
        host = await serveGuardedHttp({ embedded, token }); checkTime();
        writeFileSync(readyPath, JSON.stringify({ schemaVersion: 1, at: new Date().toISOString(), runID,
            batch: admitted.binding.batch, sourceSha256: admitted.sourceSha256, url: host.url, startupTimeoutMs: 20000,
            consentMode: consentMode ?? 'decline',
            node, helper, supervisorJobPids: ownership.supervisorJobPids, query: ownership.query,
            bearerSha256: createHash('sha256').update(token).digest('hex') }) + '\n', { flag: 'wx' });
        const closed = host.closed.finally(closeLog); void closed.catch(() => {});
        const served = host;
        return { closed, close: async () => { try { await served.close(); } finally { closeLog(); } } };
    } catch (error) {
        try { if (host) await host.close(); else await embedded?.server.close(); } finally { closeLog(); }
        throw error;
    } finally { clearTimeout(timer); }
}

async function main() {
    const token = process.env.NF_PROXY_HTTP_BEARER ?? '';
    delete process.env.NF_PROXY_HTTP_BEARER; // Never inherited by the ownership or native children.
    if (process.env.COMPUTER_USE_REQUIRE_APPROVAL !== 'true') throw new Error('Native approval policy must remain enabled');
    const child = await startGuardedHttpChild({ root: resolve(fileURLToPath(new URL('../../', import.meta.url))),
        source: process.env.NF_PROXY_BATCH_AUTHORIZATION ?? '', runID: process.env.NF_PROXY_BATCH_RUN_ID ?? '', token,
        probeRoot: process.env.NF_PROXY_HTTP_PROBE_ROOT ?? '', powershell: process.env.NF_PROXY_OWNERSHIP_POWERSHELL,
        consentMode: process.env.NF_PROXY_HTTP_CONSENT_MODE });
    const stop = () => { void child.close().catch(() => { process.exitCode = 1; }); };
    process.once('SIGINT', stop); process.once('SIGTERM', stop);
    try { await child.closed; }
    finally { process.off('SIGINT', stop); process.off('SIGTERM', stop); }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    try { await main(); }
    catch { console.error('[guarded-http-child] Startup or owned session failed'); process.exitCode = 1; }
}
