// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { resolve } from 'node:path';
import { readBatchAuthorization, claimBatchAuthorization, readClaimedBatchAuthorization } from './batch-authorization.mjs';

import { fixture, write, root } from './batch-test-fixture.mjs';

test('fresh file/lease/owned binding claims once and is independently revalidated by the MCP child', () => {
    const f = fixture();
    try {
        const authorization = readBatchAuthorization(f.path, { root, runID: 'run-one', now: f.now });
        assert.deepEqual(authorization.binding.ownedPids, [101]);
        claimBatchAuthorization(authorization);
        assert.deepEqual(readClaimedBatchAuthorization(f.path, { root, runID: 'run-one', now: f.now }).binding, authorization.binding);
        assert.throws(() => claimBatchAuthorization(authorization));
        assert.throws(() => readClaimedBatchAuthorization(f.path, { root, runID: 'another-run', now: f.now }));
    } finally { f.close(); }
});

test('revocation and elapsed original game bound invalidate a previously claimed batch', () => {
    const f = fixture();
    try {
        claimBatchAuthorization(readBatchAuthorization(f.path, { root, runID: 'run', now: f.now }));
        assert.throws(() => readClaimedBatchAuthorization(f.path, { root, runID: 'run', now: f.now + 480000 }));
        f.lease.authorized = false; f.refresh();
        assert.throws(() => readClaimedBatchAuthorization(f.path, { root, runID: 'run', now: f.now }));
    } finally { f.close(); }
});

for (const change of ['array-batch', 'unknown-field', 'foreign-target-file', 'wrong-hash', 'foreign-scenario']) test(change + ' refuses before any claim or client effect', () => {
    const f = fixture();
    try {
        const auth = /** @type {any} */ (f.auth);
        if (change === 'array-batch') auth.batch = ['B1'];
        if (change === 'unknown-field') auth.extra = true;
        if (change === 'foreign-target-file') auth.ownedReceiptPath = f.path;
        if (change === 'wrong-hash') auth.planSha256 = '0'.repeat(64);
        if (change === 'foreign-scenario') auth.scenario = dirnameOutside();
        write(f.path, auth);
        assert.throws(() => readBatchAuthorization(f.path, { root, runID: 'run', now: f.now }));
    } finally { f.close(); }
});
function dirnameOutside() { return resolve(root, '.tmp'); }

test('missing fixture binding and missing controller identity cannot manufacture a valid launch', () => {
    for (const change of ['fixture', 'controller']) {
        const f = fixture();
        try {
            if (change === 'fixture') { delete /** @type {any} */ (f.lease).fixtureReceiptSha256; delete /** @type {any} */ (f.controller).fixtureReceiptSha256; }
            else delete /** @type {any} */ (f.controller).controllerPid;
            write(f.controllerPath, f.controller); f.refresh();
            assert.throws(() => readBatchAuthorization(f.path, { root, runID: 'run', now: f.now }));
        } finally { f.close(); }
    }
});
