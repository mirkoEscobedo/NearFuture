// @ts-check
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

/** @param {Record<string, {result?: string}>} jobs */
function qualify(jobs) {
    const result = spawnSync(process.execPath, ['tools/delivery/qualification.mjs'], {
        cwd: new URL('../..', import.meta.url),
        env: { ...process.env, NF_REQUIRED_JOBS: JSON.stringify(jobs) },
        encoding: 'utf8', timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    return result;
}

function priorJobs() {
    return Object.fromEntries([
        'public-build', 'public-rust', 'windows-interop', 'release-build', 'profile-smoke', 'operator-public',
    ].map(name => [name, { result: 'success' }]));
}

test('managed lifecycle execution is required alongside every previously required group', () => {
    assert.equal(qualify(priorJobs()).status, 1, 'Missing managed execution cannot qualify');
    for (const result of ['failure', 'cancelled', 'skipped', 'queued', undefined]) {
        assert.equal(qualify({ ...priorJobs(), 'managed-public': { result } }).status, 1);
    }
});

test('the complete successful profile includes managed library evidence', () => {
    const result = qualify({ ...priorJobs(), 'managed-public': { result: 'success' } });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Qualified checks: headless-foundation-v1/);
});
