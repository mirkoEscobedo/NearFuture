// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { directFixture } from './direct-test-fixture.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from './batch-authorization.mjs';
import { startGuardedHttpChild } from './guarded-http-child.mjs';

/** The OS identity probe and native factory are the only replaced boundaries. */
function fixture() {
    const original = directFixture();
    const { root, source, runID, powershell } = original.options;
    claimBatchAuthorization(readBatchAuthorization(source, { root, runID }));
    const output = resolve(root, '.tmp/starsector-proxy', runID);
    mkdirSync(output);
    const probeRoot = mkdtempSync(resolve(root, '.tmp/starsector-proxy/scenarios/synthetic-binding-probe-'));
    let probes = 0, sessions = 0;
    const expected = JSON.parse(readFileSync(resolve(original.scenario, 'plan.json'), 'utf8')).fixtureBinding;
    const dependencies = {
        probe: async (/** @type {any} */ options) => {
            probes++;
            assert.deepEqual(options.fixtureBinding, expected);
            return { node: { pid: process.pid, parentPid: process.ppid,
                creationTime: new Date(Date.now() - process.uptime() * 1000).toISOString(), executable: process.execPath, sessionId: 1 },
            helper: { pid: 555, parentPid: process.pid, creationTime: new Date().toISOString(), executable: powershell, sessionId: 1 },
            supervisorJobPids: [process.pid, 555], query: 'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)' };
        },
        sessionFactory: () => {
            sessions++;
            return /** @type {any} */ ({ dispatch: async () => ({ content: [{ type: 'text', text: '{}' }] }) });
        },
    };
    return { original, expected, dependencies, options: { root, source, runID, powershell, probeRoot, token: 'a'.repeat(64) },
        probes: () => probes, sessions: () => sessions };
}

test('child independently resolves frozen fixture binding before its OS probe and native session', async () => {
    const current = fixture();
    let child;
    try {
        child = await startGuardedHttpChild(current.options, current.dependencies);
        assert.equal(current.probes(), 1);
        assert.equal(current.sessions(), 1);
    } finally { await child?.close(); await current.original.close(); }
});

test('fixture changed during the OS probe refuses before native session creation', async () => {
    const current = fixture(), probe = current.dependencies.probe;
    current.dependencies.probe = async options => {
        const identities = await probe(options);
        writeFileSync(current.expected.path, readFileSync(current.expected.path, 'utf8') + '\n');
        return identities;
    };
    try {
        await assert.rejects(startGuardedHttpChild(current.options, current.dependencies), /fixture binding/i);
        assert.equal(current.probes(), 1);
        assert.equal(current.sessions(), 0);
    } finally { await current.original.close(); }
});

test('transitive maintained fixture source drift refuses before the OS probe', async () => {
    const current = fixture();
    try {
        writeFileSync(resolve(current.options.root, 'tools/starsector-proxy/fixture/seed-fixture.ps1'), 'changed');
        await assert.rejects(startGuardedHttpChild(current.options, current.dependencies), /source custody/i);
        assert.equal(current.probes(), 0);
        assert.equal(current.sessions(), 0);
    } finally { await current.original.close(); }
});
