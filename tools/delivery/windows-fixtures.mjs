// @ts-check
import { readFileSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** Validate the entire inventory before selecting any execution partition.
 * @param {unknown} input @param {string[]} files @param {string} partition
 * @returns {string[]} */
export function selectFixturePartition(input, files, partition) {
    const keys = ['a', 'b', 'c', 'isolated'];
    if (!input || typeof input !== 'object') throw Error('Invalid fixture partition manifest');
    const manifest = /** @type {{schemaVersion?: unknown, partitions?: Record<string, unknown>}} */ (input);
    if (manifest.schemaVersion !== 1 || !manifest.partitions ||
        JSON.stringify(Object.keys(manifest.partitions).sort()) !== JSON.stringify(keys.slice().sort()) ||
        !keys.includes(partition)) throw Error('Invalid fixture partition manifest or selection');
    const selected = [];
    for (const key of keys) {
        const group = manifest.partitions[key];
        if (!Array.isArray(group) || group.length === 0 ||
            !group.every(name => typeof name === 'string' && /^[a-z0-9-]+\.test\.mjs$/.test(name))) {
            throw Error('Invalid fixture partition entries');
        }
        selected.push(...group);
    }
    if (new Set(selected).size !== selected.length || new Set(files).size !== files.length ||
        JSON.stringify(selected.slice().sort()) !== JSON.stringify(files.slice().sort())) {
        throw Error('Fixture coverage must assign every current file exactly once');
    }
    return /** @type {string[]} */ (manifest.partitions[partition]).slice();
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    try {
        const [flag, partition, listing, ...extra] = process.argv.slice(2);
        if (flag !== '--partition' || !partition || (listing !== undefined && listing !== '--list') || extra.length) {
            throw Error('Usage: windows-fixtures.mjs --partition a|b|c|isolated [--list]');
        }
        const manifest = JSON.parse(readFileSync(new URL('../../config/ci/windows-fixture-partitions.json', import.meta.url), 'utf8'));
        const files = readdirSync(new URL('../starsector-proxy/fixture/', import.meta.url)).filter(name => name.endsWith('.test.mjs'));
        const names = selectFixturePartition(manifest, files, partition);
        if (listing === '--list') console.log(JSON.stringify(names));
        else {
            const result = spawnSync(process.execPath, ['--test',
                ...(partition === 'isolated' ? ['--test-concurrency=1'] : []),
                ...names.map(name => `tools/starsector-proxy/fixture/${name}`)], {
                cwd: fileURLToPath(new URL('../../', import.meta.url)), stdio: 'inherit',
            });
            if (result.error) throw result.error;
            process.exitCode = result.status ?? 1;
        }
    } catch (error) {
        console.error(error instanceof Error ? error.message : String(error));
        process.exitCode = 1;
    }
}
