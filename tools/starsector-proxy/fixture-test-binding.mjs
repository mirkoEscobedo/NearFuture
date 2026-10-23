// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

/** Public synthetic data for the external fixture boundary; never a native receipt.
 * @param {string} scenario */
export function syntheticFixtureBinding(scenario) {
    const path = resolve(scenario, 'fixture-binding.private.json');
    writeFileSync(path, JSON.stringify({ schemaVersion: 1, fixtureRoot: scenario,
        sourceInstallation: resolve(scenario, 'synthetic-source'),
        qualifiedRuntimeRoot: resolve(scenario, 'synthetic-runtime'), candidateHead: '0'.repeat(40),
        runtimeManifestSha256: '1'.repeat(64), presentationAgentPath: resolve(scenario, 'synthetic-agent.jar'),
        presentationAgentSha256: '2'.repeat(64), presentationClassSha256: '3'.repeat(64),
        driverRole: '/root/recovery_game_driver' }) + '\n');
    return { path, sha256: createHash('sha256').update(readFileSync(path)).digest('hex') };
}
