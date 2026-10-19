// @ts-check
import { resolve } from 'node:path';

/** @typedef {{schema: number, provider: string, writer: string}} Compatibility */
/**
 * Pure preflight only: no directory/database is opened or migrated here.
 * A future enabled writer must also retain its native ownership and filesystem checks.
 * @param {{persistence: {writes_enabled: boolean, schema: number|null, provider: string|null,
 * writer: string|null, roots: Record<string, string>}}} profile
 * @param {string} environment
 * @param {Compatibility} persisted
 */
export function admitWrite(profile, environment, persisted) {
    const policy = profile.persistence;
    if (policy.writes_enabled !== true) throw new Error('Persistent writers are disabled in this profile');
    if (!['development', 'test', 'release'].includes(environment) || !Object.hasOwn(policy.roots, environment)) {
        throw new Error('Unknown data environment');
    }
    const roots = ['development', 'test', 'release'].map(name => policy.roots[name]);
    if (roots.some(root => typeof root !== 'string' || !root.trim()) ||
        new Set(roots.map(root => resolve(root).toLowerCase())).size !== 3) {
        throw new Error('Data roots must be distinct');
    }
    if (!Number.isSafeInteger(policy.schema) || policy.schema === null || policy.schema < 1 ||
        !policy.provider || !policy.writer || persisted.schema !== policy.schema ||
        persisted.provider !== policy.provider || persisted.writer !== policy.writer) {
        throw new Error('Persisted schema/provider/writer is incompatible');
    }
    return policy.roots[environment];
}
