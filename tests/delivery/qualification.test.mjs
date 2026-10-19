// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { qualifyJobs } from '../../tools/delivery/qualification.mjs';

test('headless qualification requires every registered job to succeed', () => {
    const required = ['public-build', 'windows-interop', 'release-build', 'profile-smoke'];
    const success = Object.fromEntries(required.map(name => [name, { result: 'success' }]));
    assert.equal(qualifyJobs(required, success), true);
    for (const result of ['failure', 'cancelled', 'skipped', 'queued', undefined]) {
        assert.equal(qualifyJobs(required, { ...success, 'public-build': { result } }), false);
    }
    const { 'windows-interop': absent, ...missing } = success;
    assert.equal(qualifyJobs(required, missing), false);
    assert.equal(qualifyJobs([], {}), false);
    assert.equal(qualifyJobs(required, { ...success, unregistered: { result: 'success' } }), false);
});
