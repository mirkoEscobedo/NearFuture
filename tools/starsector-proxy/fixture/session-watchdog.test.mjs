// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const scenarios = resolve(repository, '.tmp/starsector-proxy/scenarios');
const powershell = process.env.NF_TEST_POWERSHELL ?? 'pwsh';
const quote = (/** @type {string} */ s) => "'" + s.replaceAll("'", "''") + "'";
const policy = resolve(repository, 'tools/starsector-proxy/fixture/session-watchdog.ps1');
/** This invokes only the real authority-data reader, never a controller or native call.
 * @param {string} root @param {number} requested */
function read(root, requested) {
    return spawnSync(powershell, ['-NoProfile', '-NonInteractive', '-Command',
        "$ErrorActionPreference='Stop'; . " + quote(policy) + '; Read-NativeSessionWatchdog ' + quote(root) + ' ' + requested],
        { cwd: repository, encoding: 'utf8', timeout: 15000 });
}
/** Synthetic budget data; actual lease identity/plan hashes remain protection-check's responsibility.
 * @param {any} plan @param {any} lease */
function fixture(plan, lease) {
    const root = mkdtempSync(resolve(scenarios, 'watchdog-agreement-test-'));
    writeFileSync(resolve(root, 'plan.json'), JSON.stringify(plan));
    writeFileSync(resolve(root, 'native-lane-lease.private.json'), JSON.stringify(lease));
    return { root, close() {
        const suffix = relative(scenarios, root);
        assert.ok(suffix && !suffix.startsWith('..') && !isAbsolute(suffix));
        rmSync(root, { recursive: true });
    } };
}
test('legacy plans without a watchdog keep the exact 480000 default', () => {
    const f = fixture({}, {});
    try { const r = read(f.root, 480000); assert.equal(r.status, 0, r.stderr); assert.equal(Number(r.stdout), 480000); }
    finally { f.close(); }
});
test('a two-hour safety watchdog requires and reads matching explicit plan and lease budgets', () => {
    const f = fixture({ sessionWatchdogMs: 7200000 }, { sessionWatchdogMs: 7200000 });
    try { const r = read(f.root, 7200000); assert.equal(r.status, 0, r.stderr); assert.equal(Number(r.stdout), 7200000); }
    finally { f.close(); }
});
test('missing, mismatched, malformed or out-of-range watchdog authority refuses', () => {
    /** @type {Array<[any, any, number]>} */
    const cases = [
        [{}, {}, 7200000],
        [{ sessionWatchdogMs: 7200000 }, {}, 7200000],
        [{ sessionWatchdogMs: 7200000 }, { sessionWatchdogMs: 480000 }, 7200000],
        [{ sessionWatchdogMs: '7200000' }, { sessionWatchdogMs: 7200000 }, 7200000],
        [{ sessionWatchdogMs: true }, { sessionWatchdogMs: 7200000 }, 7200000],
        [{ sessionWatchdogMs: 7200000 }, { sessionWatchdogMs: 7200000 }, 480000],
        [{ sessionWatchdogMs: 43200001 }, { sessionWatchdogMs: 43200001 }, 43200001],
    ];
    for (const [plan, lease, requested] of cases) {
        const f = fixture(plan, lease);
        try {
            const r = read(f.root, /** @type {number} */ (requested));
            assert.notEqual(r.status, 0);
            assert.match(r.stderr, /matching session watchdog|between 480000 and 43200000/);
        } finally { f.close(); }
    }
});
