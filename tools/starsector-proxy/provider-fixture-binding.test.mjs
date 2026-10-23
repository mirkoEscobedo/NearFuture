// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { directFixture } from './direct-test-fixture.mjs';
import { startOwnedHttpBatch } from './owned-http-batch.mjs';

/** @param {string} path */
const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
/** @param {string} path @param {any} value */
const write = (path, value) => writeFileSync(path, JSON.stringify(value) + '\n');
/** @param {ReturnType<typeof directFixture>} fixture @param {(plan: any) => void} change */
function revisePlan(fixture, change) {
    const planPath = resolve(fixture.scenario, 'plan.json');
    const plan = JSON.parse(readFileSync(planPath, 'utf8'));
    change(plan); write(planPath, plan);
    const source = JSON.parse(readFileSync(fixture.options.source, 'utf8'));
    source.planSha256 = sha(planPath);
    const lease = JSON.parse(readFileSync(source.leasePath, 'utf8'));
    lease.planSha256 = source.planSha256; write(source.leasePath, lease);
    source.leaseSha256 = sha(source.leasePath); write(fixture.options.source, source);
}

test('provider refuses an unbound fixture before consuming authority or spawning', async () => {
    const fixture = directFixture();
    /** @type {Awaited<ReturnType<typeof startOwnedHttpBatch>> | undefined} */
    let batch;
    try {
        revisePlan(fixture, plan => { delete plan.fixtureBinding; });
        await assert.rejects(async () => { batch = await startOwnedHttpBatch(fixture.options, { spawn: fixture.spawn }); }, /fixture binding/i);
        assert.equal(fixture.starts(), 0);
        assert.equal(existsSync(resolve(fixture.scenario, 'guard-B1-consumed.private.json')), false);
    } finally { await batch?.close(); await fixture.close(); }
});

for (const kind of ['extra-field', 'digest-drift', 'external-path', 'different-fixture']) {
    test('provider refuses ' + kind + ' before consuming authority or spawning', async () => {
        const fixture = directFixture();
        try {
            revisePlan(fixture, plan => {
                if (kind === 'extra-field') plan.fixtureBinding.unknown = true;
                if (kind === 'digest-drift') plan.fixtureBinding.sha256 = 'f'.repeat(64);
                if (kind === 'external-path') plan.fixtureBinding.path = resolve(fixture.scenario, '..', 'foreign-binding.json');
                if (kind === 'different-fixture') {
                    const path = plan.fixtureBinding.path;
                    const binding = JSON.parse(readFileSync(path, 'utf8'));
                    binding.fixtureRoot = resolve(fixture.scenario, '..', 'other-fixture');
                    write(path, binding); plan.fixtureBinding.sha256 = sha(path);
                }
            });
            await assert.rejects(startOwnedHttpBatch(fixture.options, { spawn: fixture.spawn }), /fixture binding/i);
            assert.equal(fixture.starts(), 0);
            assert.equal(existsSync(resolve(fixture.scenario, 'guard-B1-consumed.private.json')), false);
        } finally { await fixture.close(); }
    });
}

test('a correctly bound provider retains the original claimed authority and terminal cleanup', async () => {
    const fixture = directFixture();
    let batch;
    try {
        batch = await startOwnedHttpBatch(fixture.options, { spawn: fixture.spawn });
        assert.equal(fixture.starts(), 1);
        assert.equal(existsSync(resolve(fixture.scenario, 'guard-B1-consumed.private.json')), true);
        const terminal = await batch.close();
        assert.equal(terminal.verdict, 'PASS');
        assert.equal(terminal.terminalObserved, true);
        assert.equal(terminal.steps[0].processOwnership, 'windows-job-object');
    } finally { await batch?.close(); await fixture.close(); }
});
