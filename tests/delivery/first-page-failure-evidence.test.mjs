// @ts-check
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { createFirstPageFailureObserver } from '../../tools/starsector-proxy/fixture/first-page-failure-evidence.mjs';

const repository = fileURLToPath(new URL('../../', import.meta.url));
const temporary = resolve(repository, '.tmp');
function fixture() {
    const scenarios = resolve(temporary, 'starsector-proxy/scenarios');
    mkdirSync(scenarios, { recursive: true });
    const root = mkdtempSync(resolve(scenarios, 'seed-target-test-'));
    const parent = mkdtempSync(resolve(temporary, 'near-future-first-page-failures-'));
    writeFileSync(resolve(parent, 'caller-receipt.json'), JSON.stringify({ schemaVersion: 1,
        kind: 'owned-public-first-page-child-evidence', syntheticOnly: true }));
    const data = {
        'fixture-binding.private.json': JSON.stringify({ schemaVersion: 1, fixtureRoot: root,
            sourceInstallation: resolve(root, 'original'), qualifiedRuntimeRoot: resolve(root, 'runtime'),
            candidateHead: '1234567890123456789012345678901234567890' }),
        'protected-fixture-receipt.private.json': JSON.stringify({ schemaVersion: 1, phase: 'FROZEN_PREPARATION_ONLY',
            installation: resolve(root, 'installation'), originalWrites: false, nativeLaunched: false, gameLaunched: false }),
        'protected-original-snapshot.private.json': '[{"generated":"only"}]\r\n',
        'launch-supervisor.private.json': 'Controlled pre-effect sentinel',
    };
    for (const [name, value] of Object.entries(data)) writeFileSync(resolve(root, name), value);
    return { root, parent, data, close() {
        for (const path of [root, parent]) {
            const part = relative(temporary, path);
            if (!part || part.startsWith('..') || isAbsolute(part)) throw Error('Test cleanup escaped temporary root');
            rmSync(path, { recursive: true });
        }
    } };
}
const incomplete = () => Object.freeze({ args: Object.freeze(['-File', 'synthetic-controller.ps1']),
    status: /** @type {null} */ (null), signal: 'SIGTERM', pid: 41,
    error: Object.freeze({ name: 'Error', code: 'ETIMEDOUT', message: 'spawnSync fixture ETIMEDOUT' }),
    stdout: 'partial out\r\nα\0', stderr: 'partial error\n' });

test('an incomplete child retains exact captured strings and only four generated metadata files', () => {
    const f = fixture();
    try {
        const capture = createFirstPageFailureObserver(f.parent, f.root);
        const indexPath = capture(incomplete());
        const index = JSON.parse(readFileSync(indexPath, 'utf8'));
        assert.equal(index.kind, 'FIRST_PAGE_SYNTHETIC_CHILD_FAILURE_EVIDENCE');
        assert.equal(index.complete, true);
        assert.equal(index.syntheticOnly, true);
        assert.equal(index.sourceQualified, false);
        assert.equal(index.nativeAuthority, false);
        assert.equal(index.child.status, null);
        assert.equal(index.child.error.code, 'ETIMEDOUT');
        assert.equal(readFileSync(resolve(indexPath, '..', 'stdout.txt'), 'utf8'), 'partial out\r\nα\0');
        assert.equal(readFileSync(resolve(indexPath, '..', 'stderr.txt'), 'utf8'), 'partial error\n');
        assert.deepEqual(index.facts.map((/** @type {{name:string}} */ row) => row.name), Object.keys(f.data));
        for (const [name, value] of Object.entries(f.data)) assert.equal(readFileSync(resolve(indexPath, '..', name), 'utf8'), value);
        assert.deepEqual(readdirSync(f.parent).filter(name => name !== 'caller-receipt.json').length, 1);
        assert.throws(() => capture(incomplete()), /REFUSED/);
        assert.equal(readdirSync(f.parent).length, 2, 'A consumed observer cannot create another capture');
    } finally { f.close(); }
});

test('a successful terminal result creates no child evidence', async () => {
    const { assertTerminalPowerShellResult } = await import('../../tools/starsector-proxy/fixture/powershell-child-result.mjs');
    const f = fixture();
    try {
        const capture = createFirstPageFailureObserver(f.parent, f.root);
        assertTerminalPowerShellResult({ status: 1, signal: null, pid: 42, stdout: '', stderr: 'expected' }, [], capture);
        assert.deepEqual(readdirSync(f.parent), ['caller-receipt.json']);
    } finally { f.close(); }
});

for (const fault of ['unowned parent', 'nonempty parent', 'live marker', 'native effect marker', 'oversized output', 'oversized metadata']) {
    test('failure capture refuses ' + fault + ' before child evidence writes', () => {
        const f = fixture();
        try {
            if (fault === 'unowned parent') writeFileSync(resolve(f.parent, 'caller-receipt.json'), '{"syntheticOnly":false}');
            if (fault === 'nonempty parent') writeFileSync(resolve(f.parent, 'do-not-overwrite.txt'), 'owner');
            if (fault === 'live marker') writeFileSync(resolve(f.root, 'fixture-binding.private.json'), JSON.stringify({ candidateHead: 'f'.repeat(40) }));
            if (fault === 'native effect marker') writeFileSync(resolve(f.root, 'protected-fixture-receipt.private.json'), '{"nativeLaunched":true}');
            if (fault === 'oversized metadata') writeFileSync(resolve(f.root, 'protected-original-snapshot.private.json'), Buffer.alloc(1024 * 1024 + 1));
            const value = fault === 'oversized output' ? { ...incomplete(), stdout: 'x'.repeat(2 * 1024 * 1024 + 1) } : incomplete();
            assert.throws(() => createFirstPageFailureObserver(f.parent, f.root)(value), /REFUSED/);
            assert.equal(readdirSync(f.parent).filter(name => name.startsWith('first-page-child-')).length, 0);
            if (fault === 'nonempty parent') assert.equal(readFileSync(resolve(f.parent, 'do-not-overwrite.txt'), 'utf8'), 'owner');
        } finally { f.close(); }
    });
}

test('capture preserves null versus empty output and rejects linked metadata', async () => {
    const { linkSync } = await import('node:fs');
    const f = fixture();
    try {
        const capture = createFirstPageFailureObserver(f.parent, f.root);
        const indexPath = capture({ ...incomplete(), stdout: null, stderr: '' });
        const index = JSON.parse(readFileSync(indexPath, 'utf8'));
        assert.equal(index.child.stdout, null);
        assert.equal(index.child.stderr.bytes, 0);
        assert.equal(readFileSync(resolve(indexPath, '..', 'stderr.txt')).length, 0);
    } finally { f.close(); }
    const linked = fixture();
    try {
        linkSync(resolve(linked.root, 'launch-supervisor.private.json'), resolve(linked.root, 'other-hardlink'));
        assert.throws(() => createFirstPageFailureObserver(linked.parent, linked.root)(incomplete()), /REFUSED/);
        assert.deepEqual(readdirSync(linked.parent), ['caller-receipt.json']);
    } finally { linked.close(); }
});

test('capture refuses foreign fixture roots and linked directories', async () => {
    const { symlinkSync, unlinkSync } = await import('node:fs');
    const { randomUUID } = await import('node:crypto');
    const f = fixture();
    const linked = resolve(temporary, 'starsector-proxy/scenarios/seed-target-test-link' + randomUUID().replaceAll('-', ''));
    try {
        assert.throws(() => createFirstPageFailureObserver(f.parent, f.parent), /REFUSED/);
        symlinkSync(f.root, linked, 'junction');
        try { assert.throws(() => createFirstPageFailureObserver(f.parent, linked), /REFUSED/); }
        finally { unlinkSync(linked); }
        assert.deepEqual(readdirSync(f.parent), ['caller-receipt.json']);
    } finally { f.close(); }
});

test('unassigned error fields and nonfailure records never become public evidence', () => {
    for (const value of [
        { ...incomplete(), error: { ...incomplete().error, unassigned: 'must not export' } },
        { ...incomplete(), status: 0 },
    ]) {
        const f = fixture();
        try {
            // Exercise runtime input validation despite the deliberately invalid test value.
            assert.throws(() => createFirstPageFailureObserver(f.parent, f.root)(/** @type {any} */ (value)), /REFUSED/);
            assert.deepEqual(readdirSync(f.parent), ['caller-receipt.json']);
        } finally { f.close(); }
    }
});

test('the real PowerShell helper retains an incomplete spawn before its original assertion', async () => {
    const { powershell } = await import('../../tools/starsector-proxy/fixture/seed-test-support.mjs');
    const f = fixture();
    const prior = process.env.NF_TEST_POWERSHELL;
    try {
        process.env.NF_TEST_POWERSHELL = resolve(f.root, 'deliberately-absent-pwsh.exe');
        const observe = createFirstPageFailureObserver(f.parent, f.root);
        assert.throws(() => powershell(['-File', 'synthetic-controller.ps1'], observe),
            error => error instanceof assert.AssertionError && error.actual === null && /ENOENT/.test(error.message));
        const directory = readdirSync(f.parent).find(name => name.startsWith('first-page-child-'));
        assert.ok(directory);
        const index = JSON.parse(readFileSync(resolve(f.parent, directory, 'index.actual.json'), 'utf8'));
        assert.equal(index.child.error.code, 'ENOENT');
        assert.equal(index.child.status, null);
        assert.deepEqual(index.child.stdout, { kind: 'absent' });
        assert.deepEqual(index.child.stderr, { kind: 'absent' });
        assert.deepEqual(index.child.args, ['-File', 'synthetic-controller.ps1']);
    } finally {
        if (prior === undefined) delete process.env.NF_TEST_POWERSHELL;
        else process.env.NF_TEST_POWERSHELL = prior;
        f.close();
    }
});
