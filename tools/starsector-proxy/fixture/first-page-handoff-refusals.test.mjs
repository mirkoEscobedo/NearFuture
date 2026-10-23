// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import { completedSeedLaunch } from './campaign-seed-handoff-test-support.mjs';
import { seedInputSession, claimSeededLauncher } from './campaign-input-test-support.mjs';
import { campaignLaunchFixture } from './campaign-launch-test-support.mjs';
import { json } from './seed-test-support.mjs';
import { readBatchAuthorization, claimBatchAuthorization } from '../batch-authorization.mjs';
/** @param {any} f */
function refreshInspection(f) {
    const reference = { path: f.receiptPath, sha256: json(f.receiptPath, f.receipt).sha256.toLowerCase() };
    f.plan.b1MenuReceipt = f.lease.b1MenuReceipt = reference;
    f.auth.planSha256 = f.lease.planSha256 = json(f.phasePlanPath, f.plan).sha256;
    f.auth.leaseSha256 = json(f.phaseLeasePath, f.lease).sha256;
    json(f.source, f.auth);
}
/** @type {Array<[string, (f: any) => void]>} */
const faults = [
    ['unknown provider closure', f => { f.receipt.provider.providerReaped = false; }],
    ['wrong inspected scene', f => { f.receipt.inspection.state = 'FIRST_WIZARD_PAGE'; }],
    ['legacy Load Game coordinate field', f => {
        f.receipt.inspection.loadCoordinate = f.receipt.inspection.newGameCoordinate;
        delete f.receipt.inspection.newGameCoordinate;
    }],
    ['missing main-menu original', f => { unlinkSync(f.receipt.frame.pngPath); }],
    ['changed B1 claim', f => { json(resolve(f.scenario, 'guard-B1-consumed.private.json'), {}); }],
    ['changed original launch bytes', f => { json(f.planPath, { ...f.plan, altered: true }); }],
];
for (const [name, change] of faults) test('first-page handoff refuses ' + name + ' before an E1 claim', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    try {
        f = await completedSeedLaunch({ inspection: true });
        change(f); refreshInspection(f);
        const state = f.ledger();
        const source = f.source, root = f.root;
        assert.throws(() => readBatchAuthorization(source, { root, runID: 'first-page-handoff-refused' }));
        assert.deepEqual(f.ledger(), state);
    } finally {
        await f?.close(); names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

for (const forbidden of ['wrong coordinate', 'key', 'exhausted budget']) test('first-page guard refuses ' + forbidden + ' without a native effect', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const old = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    /** @type {Awaited<ReturnType<typeof completedSeedLaunch>> | undefined} */ let f;
    /** @type {ReturnType<typeof seedInputSession> | undefined} */ let h;
    /** @type {string[]} */ const effects = [];
    try {
        f = await completedSeedLaunch({ inspection: true });
        const runID = 'first-page-forbidden-input';
        claimBatchAuthorization(readBatchAuthorization(f.source, { root: f.root, runID }));
        if (forbidden === 'exhausted budget') {
            const state = f.ledger(); state.history.reservedActions = 8;
            json(resolve(f.scenario, 'guard-launch-history.private.json'), state);
        }
        h = seedInputSession({ ...f, runID }, tool => effects.push(tool), { window: () => ({
            pid: 101, windowId: 21, title: 'Starsector', bundleId: 'java.exe', isFocused: true, isOnScreen: true,
            bounds: { x: 10, y: 10, width: 480, height: 220 } }) });
        await h.before(); const active = h;
        const args = { target_window_id: 21, target_app: 'java.exe', focus_strategy: 'strict' };
        await assert.rejects(() => forbidden === 'key' ? active.call('key', { ...args, text: 'escape', repeat: 1 }) :
            active.call('left_click', { ...args, coordinate: forbidden === 'wrong coordinate' ? [41, 80] : [40, 80] }));
        assert.deepEqual(effects, []);
        assert.equal(f.ledger().uncertain, true);
    } finally {
        await h?.embedded.server.close(); await f?.close();
        names.forEach((name, i) => { if (old[i] === undefined) delete process.env[name]; else process.env[name] = old[i]; });
    }
});

test('first-page schema4 cannot select E2 even with a self-consistent source descriptor', async () => {
    // No provider or image setup is needed: the closed public authorization rejects E2 first.
    const generated = campaignLaunchFixture({ inspection: true });
    try {
        const b1 = claimSeededLauncher(generated, 'first-page-no-e2', false);
        const source = JSON.parse(readFileSync(b1.source, 'utf8'));
        const path = resolve(generated.scenario, 'E2-authorization.private.json');
        json(path, { ...source, batch: 'E2' });
        assert.throws(() => readBatchAuthorization(path, { root: generated.root, runID: 'first-page-no-e2' }), /closed V2 authorization/);
    } finally { generated.close(); }
});
