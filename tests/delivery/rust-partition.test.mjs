// @ts-check
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { cargoArguments, partitionWorkspace, validateWorkflow } from '../../tools/delivery/rust-partition.mjs';

const root = new URL('../..', import.meta.url);
const workflow = readFileSync(new URL('.github/workflows/synthetic.yml', root), 'utf8');

test('partition selection accounts for the actual locked Cargo workspace and target inventory', () => {
    const metadata = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1', '--locked', '--offline'], {
        cwd: root, encoding: 'utf8', timeout: 30_000,
    });
    assert.equal(metadata.status, 0, metadata.stderr);
    const actual = JSON.parse(metadata.stdout);
    /** @type {{name:string,id:string,targets:{kind:string[],doctest:boolean}[]}[]} */
    const packages = actual.packages.filter(/** @param {{id:string}} pkg */ pkg => actual.workspace_members.includes(pkg.id));
    const names = packages.map(pkg => pkg.name);
    const selected = [];
    for (const suite of ['store', 'peers', 'remainder']) {
        const result = spawnSync(process.execPath, ['tools/delivery/rust-partition.mjs', '--suite', suite, '--plan'], {
            cwd: root, encoding: 'utf8', timeout: 30_000,
        });
        assert.equal(result.status, 0, result.stderr);
        const plan = JSON.parse(result.stdout);
        assert.deepEqual(plan.workspace, packages);
        selected.push(...plan.packages);
        assert.equal(plan.cells.length, 6);
        assert.deepEqual(plan.cargo.slice(-4), ['--locked', '--offline', '--', '--test-threads=1']);
        assert.ok(plan.workspace.some(/** @param {{targets:{doctest:boolean}[]}} pkg */ pkg => pkg.targets.some(target => target.doctest)));
    }
    assert.deepEqual(selected.sort(), names.sort());
    assert.equal(new Set(selected).size, names.length);
});

test('missing, repeated and unknown required input cannot silently lose a package or select tests', () => {
    const members = ['nf-store', 'nf-identity', 'nf-ipc', 'nf-transport', 'near-future'];
    const partition = partitionWorkspace([...members, 'future-workspace-member']);
    assert.deepEqual(partition.remainder, ['future-workspace-member', 'near-future']);
    for (const invalid of [[], members.slice(1), [...members, 'nf-store'], members.slice(0, 4)])
        assert.throws(() => partitionWorkspace(invalid));
    for (const suite of ['', 'unknown', 'store --all-targets']) assert.throws(() => cargoArguments(suite));
    for (const args of [[], ['--suite'], ['--suite', 'unknown'], ['--suite', 'store', '--ignored'], ['--plan']]) {
        const result = spawnSync(process.execPath, ['tools/delivery/rust-partition.mjs', ...args], {
            cwd: root, encoding: 'utf8', timeout: 10_000,
        });
        assert.equal(result.status, 1);
        assert.equal(result.stdout, '', 'Invalid selection must not reach planning or Cargo execution');
        assert.match(result.stderr, /Rust partition refused/);
    }
});

test('all six actual required workflow cells and unfiltered Cargo selections are mandatory', () => {
    assert.deepEqual(validateWorkflow(workflow), [
        { platform: 'ubuntu-latest', suite: 'store' }, { platform: 'ubuntu-latest', suite: 'peers' },
        { platform: 'ubuntu-latest', suite: 'remainder' }, { platform: 'windows-latest', suite: 'store' },
        { platform: 'windows-latest', suite: 'peers' }, { platform: 'windows-latest', suite: 'remainder' },
    ]);
    for (const [before, after] of [
        ['store, peers, remainder', 'store, peers'], ['store, peers, remainder', 'store, peers, peers'],
        ['store, peers, remainder', 'store, peers, unknown'], ['ubuntu-latest, windows-latest', 'ubuntu-latest'],
        ['ubuntu-latest, windows-latest', 'ubuntu-latest, ubuntu-latest'], ['fail-fast: false', 'fail-fast: true'],
        ['timeout-minutes: 20', 'timeout-minutes: 25'], ['--suite ${{ matrix.suite }}', '--suite store'],
    ]) assert.throws(() => validateWorkflow(workflow.replaceAll(before, after)));
    assert.deepEqual(cargoArguments('store'), ['test', '-p', 'nf-store', '--locked', '--offline', '--', '--test-threads=1']);
    assert.deepEqual(cargoArguments('peers'), ['test', '-p', 'nf-identity', '-p', 'nf-ipc', '-p', 'nf-transport', '--locked', '--offline', '--', '--test-threads=1']);
    assert.deepEqual(cargoArguments('remainder'), ['test', '--workspace', '--exclude', 'nf-store', '--exclude', 'nf-identity', '--exclude', 'nf-ipc', '--exclude', 'nf-transport', '--locked', '--offline', '--', '--test-threads=1']);
});
