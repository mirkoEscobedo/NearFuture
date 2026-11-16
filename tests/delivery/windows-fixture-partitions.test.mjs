// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { selectFixturePartition } from '../../tools/delivery/windows-fixtures.mjs';

const files = ['a.test.mjs', 'b.test.mjs', 'c.test.mjs', 'isolated.test.mjs'];
const manifest = { schemaVersion: 1, partitions: {
    a: ['a.test.mjs'], b: ['b.test.mjs'], c: ['c.test.mjs'], isolated: ['isolated.test.mjs'],
} };

test('a fixture partition refuses lost, overlapping, stale and newly unassigned coverage', () => {
    assert.deepEqual(selectFixturePartition(manifest, files, 'b'), ['b.test.mjs']);
    assert.throws(() => selectFixturePartition(manifest, [...files, 'new.test.mjs'], 'a'), /coverage/);
    assert.throws(() => selectFixturePartition(manifest, files.slice(1), 'a'), /coverage/);
    const overlap = structuredClone(manifest);
    overlap.partitions.b.push('a.test.mjs');
    assert.throws(() => selectFixturePartition(overlap, files, 'c'), /coverage/);
    const missing = structuredClone(manifest);
    missing.partitions.a = [];
    assert.throws(() => selectFixturePartition(missing, files, 'b'), /partition/);
    assert.throws(() => selectFixturePartition(manifest, files, 'unknown'), /partition/);
});

test('the named profile refuses an absent or unsuccessful Windows fixture matrix', async () => {
    const { readFileSync } = await import('node:fs');
    const { spawnSync } = await import('node:child_process');
    const old = ['public-build', 'public-rust', 'windows-interop', 'release-build', 'profile-smoke', 'operator-public'];
    const jobs = Object.fromEntries(old.map(name => [name, { result: 'success' }]));
    const run = (/** @type {Record<string, {result?: string}>} */ supplied) => spawnSync(process.execPath,
        ['tools/delivery/qualification.mjs'], { cwd: new URL('../..', import.meta.url),
            env: { ...process.env, NF_REQUIRED_JOBS: JSON.stringify(supplied) }, encoding: 'utf8', timeout: 10000 });
    assert.equal(run(jobs).status, 1, 'The old successful inventory cannot hide missing fixture runners');
    for (const result of ['failure', 'cancelled', 'skipped', undefined]) {
        assert.equal(run({ ...jobs, 'windows-fixtures': { result } }).status, 1);
    }
    assert.equal(run({ ...jobs, 'windows-fixtures': { result: 'success' } }).status, 0);
    const workflow = readFileSync(new URL('../../.github/workflows/synthetic.yml', import.meta.url), 'utf8');
    assert.match(workflow, /needs: \[[^\n]*windows-fixtures[^\n]*\]/);
});

test('the production CLI lists the complete current inventory and refuses foreign selections', async () => {
    const { readFileSync, readdirSync } = await import('node:fs');
    const { spawnSync } = await import('node:child_process');
    const cwd = new URL('../..', import.meta.url);
    const run = (/** @type {string} */ partition) => spawnSync(process.execPath,
        ['tools/delivery/windows-fixtures.mjs', '--partition', partition, '--list'],
        { cwd, encoding: 'utf8', timeout: 10000 });
    const actual = readdirSync(new URL('../../tools/starsector-proxy/fixture/', import.meta.url))
        .filter(name => name.endsWith('.test.mjs')).sort();
    const selected = ['a', 'b', 'c', 'isolated'].flatMap(partition => {
        const result = run(partition);
        assert.equal(result.status, 0, result.stderr);
        return JSON.parse(result.stdout);
    });
    assert.deepEqual(selected.sort(), actual);
    assert.equal(new Set(selected).size, selected.length);
    const refused = run('../a');
    assert.equal(refused.status, 1);
    assert.match(refused.stderr, /partition/);
    const source = readFileSync(new URL('../../tools/delivery/windows-fixtures.mjs', import.meta.url), 'utf8');
    assert.doesNotMatch(source, /--test-name-pattern|--test-skip-pattern/);
});
