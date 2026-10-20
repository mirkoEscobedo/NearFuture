// @ts-check
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

function successfulJobs() {
    return Object.fromEntries([
        'public-build', 'public-rust', 'windows-interop', 'release-build', 'profile-smoke', 'operator-public', 'managed-public',
    ].map(name => [name, { result: 'success' }]));
}

/** @param {Record<string, {result?: string}>} jobs */
function qualifyProfile(jobs) {
    const result = spawnSync(process.execPath, ['tools/delivery/qualification.mjs'], {
        cwd: new URL('../..', import.meta.url),
        env: { ...process.env, NF_REQUIRED_JOBS: JSON.stringify(jobs) },
        encoding: 'utf8', timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    return result;
}

test('the named profile requires the separately executed Rust matrix', () => {
    const missing = successfulJobs();
    delete missing['public-rust'];
    assert.equal(qualifyProfile(missing).status, 1, 'Missing Rust execution cannot qualify');
    for (const result of ['failure', 'cancelled', 'skipped', 'queued', undefined]) {
        assert.equal(qualifyProfile({ ...successfulJobs(), 'public-rust': { result } }).status, 1);
    }
});

test('the complete successful partition qualifies through the public profile CLI', () => {
    const result = qualifyProfile(successfulJobs());
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Qualified checks: headless-foundation-v1/);
});
