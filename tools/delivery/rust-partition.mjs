// @ts-check
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const named = { store: ['nf-store'], peers: ['nf-identity', 'nf-ipc', 'nf-transport'] };
export const suites = ['store', 'peers', 'remainder'];
const platforms = ['ubuntu-latest', 'windows-latest'];

/** @param {string[]} members */
export function partitionWorkspace(members) {
    if (!members.length || members.some(name => typeof name !== 'string' || !name.trim()) || new Set(members).size !== members.length)
        throw new Error('Invalid workspace membership');
    const assigned = Object.values(named).flat();
    if (new Set(assigned).size !== assigned.length || assigned.some(name => !members.includes(name)))
        throw new Error('Missing or repeated required package');
    const partition = { ...named, remainder: members.filter(name => !assigned.includes(name)).sort() };
    if (!partition.remainder.length || Object.values(partition).flat().length !== members.length)
        throw new Error('Incomplete partition');
    return partition;
}

/** @param {string} suite */
export function cargoArguments(suite) {
    if (!suites.includes(suite)) throw new Error('Invalid suite selection');
    const selection = suite === 'remainder'
        ? ['--workspace', ...Object.values(named).flat().flatMap(name => ['--exclude', name])]
        : /** @type {Record<string,string[]>} */ (named)[suite].flatMap(name => ['-p', name]);
    return ['test', ...selection, '--locked', '--offline', '--', '--test-threads=1'];
}

/** Validate the actual supported workflow matrix, not a separate disconnected registry.
 * @param {string} workflow
 */
export function validateWorkflow(workflow) {
    const job = workflow.match(/^  public-rust:\r?\n([\s\S]*?)(?=^  [a-z][\w-]*:)/m)?.[1] ?? '';
    const matrix = job.match(/^      matrix:\r?\n([\s\S]*?)(?=^    \S)/m)?.[1] ?? '';
    const keys = [...matrix.matchAll(/^        ([\w-]+):/gm)].map(match => match[1]);
    if (keys.length !== 2 || !keys.includes('os') || !keys.includes('suite')) throw new Error('Unsupported Rust matrix');
    /** @param {string} key @param {string[]} expected */
    function axis(key, expected) {
        const values = matrix.match(new RegExp(`^        ${key}: \\[([^\\]\\r\\n]+)\\]\\r?$`, 'm'))?.[1].split(',').map(value => value.trim()) ?? [];
        if (values.length !== expected.length || new Set(values).size !== values.length || expected.some(value => !values.includes(value)))
            throw new Error('Missing, duplicate or unknown matrix cell');
        return values;
    }
    const os = axis('os', platforms);
    const selected = axis('suite', suites);
    if (!/^      fail-fast: false\r?$/m.test(job) || !/^    timeout-minutes: 20\r?$/m.test(job) ||
        !/^    runs-on: \$\{\{ matrix\.os \}\}\r?$/m.test(job) ||
        !/^      - name: Run one exhaustive Rust suite\r?\n        run: node tools\/delivery\/rust-partition\.mjs --suite \$\{\{ matrix\.suite \}\}\r?$/m.test(job))
        throw new Error('Rust execution contract changed');
    return os.flatMap(platform => selected.map(suite => ({ platform, suite })));
}

function metadata() {
    const result = spawnSync('cargo', ['metadata', '--no-deps', '--format-version', '1', '--locked', '--offline'], {
        cwd: root, encoding: 'utf8', timeout: 30_000, maxBuffer: 8 * 1024 * 1024,
    });
    if (result.error || result.status !== 0) throw new Error('Cargo workspace metadata unavailable');
    const value = JSON.parse(result.stdout);
    /** @type {{id:string,name:string,targets:{name:string,kind:string[],doctest:boolean}[]}[]} */
    const packages = value.packages;
    /** @type {string[]} */
    const ids = value.workspace_members;
    const members = ids.map(id => {
        const found = packages.filter(pkg => pkg.id === id);
        if (found.length !== 1) throw new Error('Ambiguous workspace package');
        return found[0];
    });
    return members;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    try {
        const args = process.argv.slice(2);
        if (args[0] !== '--suite' || !suites.includes(args[1]) || !(args.length === 2 || (args.length === 3 && args[2] === '--plan')))
            throw new Error('Explicit suite required; optional --plan only');
        const cells = validateWorkflow(readFileSync(new URL('../../.github/workflows/synthetic.yml', import.meta.url), 'utf8'));
        const members = metadata();
        const partition = partitionWorkspace(members.map(pkg => pkg.name));
        const suite = args[1];
        const selected = /** @type {Record<string,string[]>} */ (partition)[suite];
        const cargo = cargoArguments(suite);
        console.log(JSON.stringify({ suite, packages: selected, cargo, cells, workspace: members }));
        if (args[2] !== '--plan') {
            const result = spawnSync('cargo', cargo, { cwd: root, stdio: 'inherit', timeout: 1_200_000 });
            process.exitCode = result.error ? 1 : (result.status ?? 1);
        }
    } catch {
        console.error('Rust partition refused: verify explicit suite, complete workspace and required workflow.');
        process.exitCode = 1;
    }
}
