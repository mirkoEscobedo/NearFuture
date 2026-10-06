// @ts-check
import { readdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, relative } from 'node:path';
import { spawnSync } from 'node:child_process';
import { dependencyViolations } from './boundaries.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const cargo = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'], {
    cwd: root, encoding: 'utf8', timeout: 30_000,
});
if (cargo.error || cargo.status !== 0) {
    console.error('Boundary inspection unavailable: cargo metadata failed. Install the pinned Rust toolchain.');
    process.exit(1);
}
/** @param {string} directory @param {string} extension @returns {Promise<import("./boundaries.mjs").Source[]>} */
async function sourcesUnder(directory, extension) {
    const sources = [];
    for (const item of await readdir(directory, { withFileTypes: true })) {
        const path = resolve(directory, item.name);
        if (item.isDirectory()) sources.push(...await sourcesUnder(path, extension));
        else if (item.name.endsWith(extension)) sources.push({
            path: relative(root, path).replaceAll('\\', '/'), content: await readFile(path, 'utf8'),
        });
    }
    return sources;
}
const sources = [
    ...await sourcesUnder(resolve(root, 'crates/nf-contract/src'), '.rs'),
    ...await sourcesUnder(resolve(root, 'java/contract/src/main'), '.java'),
];
const violations = dependencyViolations(JSON.parse(cargo.stdout).packages, sources);
if (violations.length) {
    console.error(violations.join('\n'));
    process.exit(1);
}
console.log('PASS: domain dependencies and sources stay free of game/network/database effects');
