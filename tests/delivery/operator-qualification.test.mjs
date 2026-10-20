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

function originalJobs() {
    return Object.fromEntries([
        'public-build', 'public-rust', 'windows-interop', 'release-build', 'profile-smoke', 'managed-public',
    ].map(name => [name, { result: 'success' }]));
}

test('operator evidence is required in addition to the original successful profile', () => {
    assert.equal(qualify(originalJobs()).status, 1, 'Absent operator execution cannot qualify');
    for (const result of ['failure', 'cancelled', 'skipped', 'queued', undefined]) {
        assert.equal(qualify({ ...originalJobs(), 'operator-public': { result } }).status, 1);
    }
});

test('successful operator and original jobs qualify through the profile CLI', () => {
    const result = qualify({ ...originalJobs(), 'operator-public': { result: 'success' } });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Qualified checks: headless-foundation-v1/);
});
