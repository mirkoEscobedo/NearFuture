// @ts-check
import { createHash } from 'node:crypto';
import { lstatSync, fstatSync, openSync, readSync, closeSync, readdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { resolve, dirname, basename, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = fileURLToPath(new URL('../../../', import.meta.url));
const scenarios = resolve(repository, '.tmp/starsector-proxy/scenarios');
const facts = ['fixture-binding.private.json', 'protected-fixture-receipt.private.json',
    'protected-original-snapshot.private.json', 'launch-supervisor.private.json'];
const callerReceipt = { schemaVersion: 1, kind: 'owned-public-first-page-child-evidence', syntheticOnly: true };
const MiB = 1024 * 1024;
const sha = (/** @type {Buffer} */ bytes) => createHash('sha256').update(bytes).digest('hex');
const need = (/** @type {unknown} */ ok) => { if (!ok) throw Error('FIRST_PAGE_CHILD_EVIDENCE_REFUSED'); };

/** Caller owns these exclusive synthetic paths; this is not a cross-user filesystem security boundary.
 * @param {string} path @param {'file'|'directory'} kind */
function ordinary(path, kind) {
    need(isAbsolute(path));
    for (let part = resolve(path);; part = dirname(part)) {
        need(!lstatSync(part).isSymbolicLink());
        if (dirname(part) === part) break;
    }
    const stat = lstatSync(path);
    need(kind === 'file' ? stat.isFile() && stat.nlink === 1 : stat.isDirectory());
    return stat;
}
/** Read at most the declared bound, rejecting changing or replaced files. @param {string} path @param {number} limit */
function boundedBytes(path, limit) {
    ordinary(path, 'file');
    const fd = openSync(path, 'r');
    try {
        const before = fstatSync(fd);
        need(before.isFile() && before.nlink === 1 && before.size <= limit);
        const buffer = Buffer.alloc(before.size + 1);
        let length = 0;
        while (length < buffer.length) {
            const count = readSync(fd, buffer, length, buffer.length - length, null);
            if (count === 0) break;
            length += count;
        }
        const after = fstatSync(fd), current = ordinary(path, 'file');
        need(length === before.size && after.size === before.size && after.mtimeMs === before.mtimeMs &&
            after.ctimeMs === before.ctimeMs && current.dev === after.dev && current.ino === after.ino);
        return buffer.subarray(0, length);
    } finally { closeSync(fd); }
}
/** @param {string} parent */
function ownedParent(parent) {
    const stat = ordinary(parent, 'directory');
    need(/^near-future-first-page-failures-[a-z0-9-]+$/i.test(basename(parent)));
    need(JSON.stringify(readdirSync(parent).sort()) === '["caller-receipt.json"]');
    const receipt = JSON.parse(boundedBytes(resolve(parent, 'caller-receipt.json'), 1024).toString('utf8'));
    need(JSON.stringify(Object.keys(receipt).sort()) === JSON.stringify(Object.keys(callerReceipt).sort()));
    need(receipt.schemaVersion === 1 && receipt.kind === callerReceipt.kind && receipt.syntheticOnly === true);
    return { dev: stat.dev, ino: stat.ino };
}
/** @param {string|null|undefined} text */
function outputBytes(text) {
    need(text === undefined || text === null || typeof text === 'string');
    if (text === undefined || text === null) return text;
    need(text.length <= 2 * MiB);
    const bytes = Buffer.from(text, 'utf8');
    need(bytes.length <= 2 * MiB && bytes.toString('utf8') === text);
    return bytes;
}

/** Only the closed generated first-page fixture can provide public failure artifacts.
 * No output is created unless an incomplete child is observed. One observer writes at most once.
 * @param {string} parent @param {string} fixtureRoot
 * @returns {(value:import('./powershell-child-result.mjs').IncompletePowerShellResult)=>string} */
export function createFirstPageFailureObserver(parent, fixtureRoot) {
    need(isAbsolute(parent) && isAbsolute(fixtureRoot));
    parent = resolve(parent); fixtureRoot = resolve(fixtureRoot);
    const owner = ownedParent(parent);
    const part = relative(scenarios, fixtureRoot);
    need(!isAbsolute(part) && /^seed-target-test-[a-z0-9]+$/i.test(part));
    ordinary(fixtureRoot, 'directory');
    let consumed = false;
    return value => {
        need(!consumed); consumed = true;
        need(value.status === null && (value.signal === null || typeof value.signal === 'string') &&
            Number.isInteger(value.pid) && value.pid >= 0 && value.pid <= 0xffffffff);
        need(Array.isArray(value.args) && value.args.length <= 32 && value.args.every(arg => typeof arg === 'string'));
        const args = [...value.args];
        need(Buffer.byteLength(JSON.stringify(args), 'utf8') <= 65536);
        need(value.error === null || (typeof value.error.name === 'string' && typeof value.error.message === 'string' &&
            (value.error.code === null || typeof value.error.code === 'string')));
        need(value.error === null || JSON.stringify(Object.keys(value.error).sort()) === '["code","message","name"]');
        const error = value.error === null ? null : { name: value.error.name, message: value.error.message, code: value.error.code };
        need(Buffer.byteLength(JSON.stringify(error), 'utf8') <= 16384);
        const stdout = outputBytes(value.stdout), stderr = outputBytes(value.stderr);
        const metadata = facts.map(name => ({ name, bytes: boundedBytes(resolve(fixtureRoot, name), MiB) }));
        const binding = JSON.parse(metadata[0].bytes.toString('utf8'));
        const protection = JSON.parse(metadata[1].bytes.toString('utf8'));
        need(binding.schemaVersion === 1 && binding.candidateHead === '1234567890123456789012345678901234567890' &&
            typeof binding.fixtureRoot === 'string' && resolve(binding.fixtureRoot) === fixtureRoot &&
            typeof binding.sourceInstallation === 'string' && resolve(binding.sourceInstallation) === resolve(fixtureRoot, 'original') &&
            typeof binding.qualifiedRuntimeRoot === 'string' && resolve(binding.qualifiedRuntimeRoot) === resolve(fixtureRoot, 'runtime'));
        need(protection.schemaVersion === 1 && protection.phase === 'FROZEN_PREPARATION_ONLY' && protection.originalWrites === false &&
            protection.nativeLaunched === false && protection.gameLaunched === false &&
            typeof protection.installation === 'string' && resolve(protection.installation) === resolve(fixtureRoot, 'installation'));
        const current = ownedParent(parent);
        need(current.dev === owner.dev && current.ino === owner.ino);
        const destination = mkdtempSync(resolve(parent, 'first-page-child-'));
        /** @param {string} name @param {Buffer} bytes */
        function save(name, bytes) {
            ordinary(destination, 'directory');
            writeFileSync(resolve(destination, name), bytes, { flag: 'wx' });
            return { name, bytes: bytes.length, sha256: sha(bytes) };
        }
        const index = { schemaVersion: 1, kind: 'FIRST_PAGE_SYNTHETIC_CHILD_FAILURE_EVIDENCE', complete: true,
            syntheticOnly: true, sourceQualified: false, nativeAuthority: false,
            encoding: 'Original Node spawnSync UTF8 strings encoded as UTF8 without BOM; not native pipe-byte custody',
            child: { args, status: value.status, signal: value.signal, pid: value.pid, error,
                stdout: stdout === undefined ? { kind: 'absent' } : stdout === null ? null : save('stdout.txt', stdout),
                stderr: stderr === undefined ? { kind: 'absent' } : stderr === null ? null : save('stderr.txt', stderr) },
            facts: metadata.map(row => save(row.name, row.bytes)) };
        const indexPath = resolve(destination, 'index.actual.json');
        writeFileSync(indexPath, JSON.stringify(index, null, 2) + '\n', { flag: 'wx', encoding: 'utf8' });
        return indexPath;
    };
}
