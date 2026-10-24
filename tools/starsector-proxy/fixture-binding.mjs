// @ts-check
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { dirname, isAbsolute, resolve } from 'node:path';

export const fixtureProbeSources = Object.freeze([
    'tools/starsector-proxy/guarded-http-ownership.ps1',
    ...['native-controller.ps1', 'protection-check.ps1', 'fixture-inputs.ps1',
        'seed-fixture.ps1', 'publish-owned-snapshot.ps1', 'session-watchdog.ps1', 'menu-launch.ps1']
        .map(name => 'tools/starsector-proxy/fixture/' + name),
]);

/** @param {string} path @param {number} limit */
function read(path, limit) {
    for (let part = resolve(path);; part = dirname(part)) {
        if (lstatSync(part).isSymbolicLink()) throw new Error('Ordinary fixture binding custody required');
        if (dirname(part) === part) break;
    }
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.size < 2 || stat.size > limit) throw new Error('Bounded fixture binding custody required');
    const bytes = readFileSync(path);
    const text = bytes.toString('utf8').replace(/^\uFEFF/, '');
    return { text, value: JSON.parse(text),
        sha256: createHash('sha256').update(bytes).digest('hex') };
}

/** Resolve only the fixture named by the already admitted frozen plan.
 * The maintained PowerShell input boundary validates licensed/runtime inputs.
 * @param {ReturnType<typeof import('./batch-authorization.mjs').readBatchAuthorization>} admitted */
export function readFixtureBinding(admitted) {
    const { scenario, planSha256 } = admitted.binding;
    const plan = read(resolve(scenario, 'plan.json'), 256 * 1024);
    const descriptor = plan.value.fixtureBinding;
    const originalPlanSha256 = 'originalPlanSha256' in admitted.binding ? admitted.binding.originalPlanSha256 : planSha256;
    if (plan.sha256 !== originalPlanSha256 || !descriptor || Array.isArray(descriptor)
        || Object.keys(descriptor).sort().join() !== 'path,sha256'
        || typeof descriptor.path !== 'string' || !isAbsolute(descriptor.path)
        || resolve(descriptor.path) !== resolve(scenario, 'fixture-binding.private.json')
        || typeof descriptor.sha256 !== 'string' || !/^[a-f0-9]{64}$/i.test(descriptor.sha256)) {
        throw new Error('Exact frozen plan fixture binding required');
    }
    const path = resolve(descriptor.path), fixture = read(path, 16384);
    const value = fixture.value;
    if (value?.schemaVersion === 3) {
        // This closed binding has scalar values only. Tokenize complete JSON strings
        // before identifying keys, so colons/escaped quotes in paths stay values.
        const keys = new Set();
        for (const token of fixture.text.matchAll(/"(?:\\.|[^"\\])*"/g)) {
            if (!fixture.text.slice(token.index + token[0].length).trimStart().startsWith(':')) continue;
            const key = JSON.parse(token[0]).toLowerCase();
            if (keys.has(key)) throw new Error('Duplicate generation binding field refused');
            keys.add(key);
        }
    }
    const fields = ['schemaVersion', 'fixtureRoot', 'sourceInstallation', 'qualifiedRuntimeRoot',
        'candidateHead', 'runtimeManifestSha256', 'presentationAgentPath', 'presentationAgentSha256',
        'presentationClassSha256', 'driverRole'];
    if (value?.schemaVersion === 2) fields.push('recordingProfileSha256');
    if (value?.schemaVersion === 3) fields.push('fixtureProfile');
    const control = plan.value.campaignControl;
    const generation = control?.schemaVersion === 3 && control.profile === 'GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE'
        && Object.keys(control).sort().join() === 'profile,schemaVersion';
    const lease = read(resolve(scenario, 'native-lane-lease.private.json'), 256 * 1024).value;
    const controller = read(resolve(scenario, 'controller-start.private.json'), 256 * 1024).value;
    if (fixture.sha256 !== descriptor.sha256.toLowerCase() || !value || Array.isArray(value)
        || ![1, 2, 3].includes(value.schemaVersion) || Object.keys(value).sort().join() !== fields.sort().join()
        || (value.schemaVersion === 3) !== generation
        || (value.schemaVersion === 3 && (value.fixtureProfile !== 'VANILLA_UNCOMPRESSED_GENERATION'
            || (admitted.binding.schemaVersion !== undefined && admitted.binding.schemaVersion !== 5)))
        || (admitted.binding.schemaVersion === 5 && !generation)
        || typeof value.fixtureRoot !== 'string' || !isAbsolute(value.fixtureRoot)
        || resolve(value.fixtureRoot) !== scenario || value.driverRole !== lease.driver
        || resolve(controller.installation ?? '') !== resolve(value.fixtureRoot, 'installation')) {
        throw new Error('Current owned scenario fixture binding required');
    }
    return { path, sha256: fixture.sha256 };
}
