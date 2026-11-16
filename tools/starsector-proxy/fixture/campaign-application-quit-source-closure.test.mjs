// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve, relative, basename } from 'node:path';
import { ownedHttpSources } from '../owned-http-batch.mjs';
import { campaignLaunchSources } from '../campaign-authority.mjs';
import { repository } from './seed-test-support.mjs';

test('application Quit proof imports are in both current executable closures and both native pin inventories', () => {
    /** @type {Set<string>} */ const found = new Set();
    /** @param {string} path */
    function visit(path) {
        if (found.has(path)) return;
        found.add(path);
        const text = readFileSync(path, 'utf8');
        for (const match of text.matchAll(/from\s+['"]([^'"]+)['"]/g)) {
            if (match[1].startsWith('.')) visit(resolve(dirname(path), match[1]));
        }
    }
    for (const name of ['seed-application-quit-profile.mjs', 'seed-scene-inspection.mjs']) {
        visit(resolve(repository, 'tools/starsector-proxy', name));
    }
    const native = ['seed-fixture.ps1', 'seed-provenance-v2.ps1'].map(name =>
        readFileSync(resolve(repository, 'tools/starsector-proxy/fixture', name), 'utf8'));
    for (const path of found) {
        const key = relative(repository, path).replaceAll('\\', '/');
        for (const inventory of [ownedHttpSources, campaignLaunchSources]) assert.ok(inventory.includes(key), key);
        for (const text of native) assert.ok(text.includes("'" + basename(path) + "'"), basename(path));
    }
});
