// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { startOwnedHttpBatch } from './owned-http-batch.mjs';
import { directFixture } from './direct-test-fixture.mjs';

test('owned provider uses one real guarded HTTP session, actual inner ID and terminal Job receipt', async () => {
    const f = directFixture(); let batch;
    try {
        batch = await startOwnedHttpBatch(f.options, { spawn: f.spawn });
        const config = JSON.parse(readFileSync(resolve(f.args()[1], '.agentic/project.json'), 'utf8'));
        assert.equal(config.overrides.workspace.commands['node:.'][0].cwd, '.');
        const result = await batch.call('doctor', {});
        assert.notEqual(result.isError, true);
        const terminal = await batch.close();
        assert.equal(terminal.verifierStatus, 0); assert.equal(terminal.steps[0].processOwnership, 'windows-job-object');
        assert.equal(terminal.steps[0].status, 0); assert.equal(terminal.steps[0].timedOut, false);
        assert.equal(terminal.steps[0].cwd, '.'); assert.equal(terminal.terminalBoundToChild, true);
        assert.equal(f.starts(), 1); assert.equal(f.kills(), 0);
        assert.deepEqual(await batch.close(), terminal);
        const call = JSON.parse(readFileSync(resolve(batch.output, 'call-001.private.json'), 'utf8'));
        assert.equal(call.outerProtocolID, 'UNKNOWN'); assert.equal(typeof call.innerProtocolID, 'number');
        assert.equal(f.args().join(' ').includes(f.token()), false);
        assert.equal(readFileSync(resolve(batch.output, 'owned-provider.stdout.private.txt'), 'utf8').includes(f.token()), false);
    } finally { await batch?.close(); await f.close(); }
});

test('ephemeral bearer split across stdout/stderr chunks is redacted only in private terminal spools', async () => {
    const f = directFixture({ spoolSecret: true }); let batch;
    try {
        batch = await startOwnedHttpBatch(f.options, { spawn: f.spawn }); await batch.close();
        for (const path of ['owned-provider.stdout.private.txt', 'owned-provider.stderr.private.txt', 'owned-provider-terminal.private.json']) {
            assert.equal(readFileSync(resolve(batch.output, path), 'utf8').includes(f.token()), false);
        }
    } finally { await batch?.close(); await f.close(); }
});

test('cancelled startup consumes the claim and awaits owned process cleanup', async () => {
    const f = directFixture({ neverReady: true }), controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 150);
    try {
        await assert.rejects(startOwnedHttpBatch({ ...f.options, signal: controller.signal }, { spawn: f.spawn }), /readiness/);
        assert.equal(f.starts(), 1); assert.equal(f.kills(), 1);
        const receipt = JSON.parse(readFileSync(resolve(f.options.root, '.tmp/starsector-proxy', f.options.runID, 'owned-provider-terminal.private.json'), 'utf8'));
        assert.ok(receipt.steps.length === 1); assert.ok(existsSync(resolve(f.scenario, 'guard-B1-consumed.private.json')));
    } finally { clearTimeout(timer); await f.close(); }
});

test('per-call 20-second budget is not extended by regular native progress', { timeout: 25000 }, async () => {
    let cancelled = false, progressed = 0;
    const f = directFixture({ dispatch: async (tool, args, signal, progress) => new Promise((done, fail) => {
        const timer = setInterval(() => progress?.({ progress: 1, message: 'still pending' }), 100);
        signal?.addEventListener('abort', () => { clearInterval(timer); cancelled = true; fail(new Error('Synthetic native call cancelled')); }, { once: true });
    }) }); let batch;
    try {
        batch = await startOwnedHttpBatch(f.options, { spawn: f.spawn }); const started = Date.now();
        await assert.rejects(batch.call('wait', { duration: 0.1 }, { onprogress: () => { progressed++; } }));
        assert.ok(Date.now() - started >= 19900); assert.ok(Date.now() - started < 24000);
        assert.ok(progressed > 1); assert.equal(cancelled, true); assert.equal(f.kills(), 0);
    } finally { await batch?.close(); await f.close(); }
});
test('source drift refuses before claim, directories or provider spawn', async () => {
    const f = directFixture();
    try {
        writeFileSync(resolve(f.options.root, 'tools/starsector-proxy/guarded-http-child.mjs'), 'drift');
        await assert.rejects(startOwnedHttpBatch(f.options, { spawn: f.spawn }), /source custody/);
        assert.equal(f.starts(), 0); assert.equal(existsSync(resolve(f.scenario, 'guard-B1-consumed.private.json')), false);
    } finally { await f.close(); }
});
test('new native runtime asset without a matching source pin refuses before claim or spawn', async () => {
    const f = directFixture();
    try {
        writeFileSync(resolve(f.options.root, 'tools/computer-use-mcp/node_modules/synthetic-runtime/native.dll'), 'unqualified new native runtime');
        await assert.rejects(startOwnedHttpBatch(f.options, { spawn: f.spawn }), /source custody/);
        assert.equal(f.starts(), 0); assert.equal(existsSync(resolve(f.scenario, 'guard-B1-consumed.private.json')), false);
    } finally { await f.close(); }
});
test('wrong readiness parent fails closed and reaps only the spawned verifier', async () => {
    const f = directFixture({ wrongParent: true });
    try {
        await assert.rejects(startOwnedHttpBatch(f.options, { spawn: f.spawn }), /readiness/);
        assert.equal(f.starts(), 1); assert.equal(f.kills(), 1);
        assert.ok(existsSync(resolve(f.scenario, 'guard-B1-consumed.private.json')));
    } finally { await f.close(); }
});
