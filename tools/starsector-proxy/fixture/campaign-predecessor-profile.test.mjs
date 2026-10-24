// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { resolve } from 'node:path';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { filePin } from './seed-test-support.mjs';
import { readBatchAuthorization } from '../batch-authorization.mjs';
import { validateBatchBinding } from '../batch-session.mjs';

test('legacy schema2 cannot select the new fixed phase-local predecessor route', () => {
    const f = campaignLaunchFixture();
    try {
        assert.throws(() => readBatchAuthorization(f.source, { root: f.root, runID: f.runID }),
            /legacy campaign cannot use launch predecessor/);
    } finally { f.close(); }
});

for (const batch of ['E1', 'E2']) test('legacy ' + batch + ' binding cannot carry the new predecessor extension', () => {
    const f = campaignLaunchFixture();
    try {
        const binding = { schemaVersion: 2, batch, runID: f.runID, scenario: f.scenario, workspaceRoot: f.root,
            planSha256: filePin(f.phasePlanPath).sha256.toLowerCase(), ownedPids: [101], campaign: f.campaign,
            sourcePath: resolve(f.scenario, batch + '-authorization.private.json'), sourceSha256: filePin(f.source).sha256.toLowerCase(),
            controllerSha256: filePin(f.controllerPath).sha256.toLowerCase(), originalPlanSha256: filePin(f.planPath).sha256.toLowerCase(),
            ...(batch === 'E2' ? { e1MenuReceipt: { path: resolve(f.scenario, 'E1-menu-inspected.private.json'),
                sha256: '0'.repeat(64), exitCoordinate: [40, 80] } } : {}) };
        assert.throws(() => validateBatchBinding(binding), /legacy campaign cannot use launch predecessor/);
    } finally { f.close(); }
});
