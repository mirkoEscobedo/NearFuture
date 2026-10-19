// @ts-check
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/**
 * A matrix job is successful only when all its matrix children succeed.
 * @param {string[]} required
 * @param {Record<string, {result?: string}>} jobs
 */
export function qualifyJobs(required, jobs) {
    return required.length > 0 && new Set(required).size === required.length &&
        Object.keys(jobs).length === required.length &&
        required.every(name => Object.hasOwn(jobs, name) && jobs[name]?.result === 'success');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    const profile = JSON.parse(readFileSync(new URL('../../config/profiles/headless-foundation.json', import.meta.url), 'utf8'));
    const jobs = JSON.parse(process.env.NF_REQUIRED_JOBS ?? '{}');
    if (!qualifyJobs(profile.required_jobs, jobs)) {
        console.error('Headless qualification refused: every registered job must succeed.');
        process.exitCode = 1;
    } else {
        console.log(`Qualified checks: ${profile.id}`);
    }
}
